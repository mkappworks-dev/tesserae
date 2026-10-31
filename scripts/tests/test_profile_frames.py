"""Run with python3 -m unittest discover -s scripts/tests."""
import json
from pathlib import Path
import runpy
import tempfile
import unittest

PROFILE = runpy.run_path(str(Path(__file__).resolve().parents[1] / 'profile-frames'))


class FrameAnalysis(unittest.TestCase):
    def test_nearest_rank_and_empty_availability(self):
        self.assertEqual(PROFILE['distribution']([4, 1, 3, 2]),
                         dict(n=4, p50=2, p95=4, p99=4, max=4))
        self.assertIsNone(PROFILE['distribution']([])['p50'])

    def test_startup_stalls_and_render_ids_are_retained(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            run = root / 'flight-1-on'
            run.mkdir()
            (run / 'frames.csv').write_text(
                'frame_id,phase,frame_ms,main_cpu_ms\n'
                '0,startup,750,3\n1,fixture,10,2\n')
            (run / 'frames.render.csv').write_text(
                'frame_id,render_schedule_wall_ms,render_thread_cpu_ms\n'
                '0,6,2\n1,5,1\n')
            PROFILE['analyze'](root)
            result = json.loads((root / 'summary.json').read_text())['flight-1-on']
            self.assertEqual(result['startup']['frame_ms']['max'], 750)
            self.assertEqual(result['fixture']['render_thread_cpu_ms']['p99'], 1)
            self.assertEqual(result['stationary']['frame_ms']['n'], 0)


FRAME_HEADER = ('frame_id,start_s,end_s,phase,frame_ms,streaming_settled,fixture_moves,'
                'simulation_step,landed,rule_holes,chunk_entities\n')
FLOW_HEADER = ('frame_id,t_s,level,generate_waiting,generate_run,generate_cpu_ms,relight_waiting,'
               'relight_run,relight_cpu_ms,mesh_waiting,mesh_run,mesh_cpu_ms,fluid_run,fluid_cpu_ms\n')


def moves_run(root, frames, flow):
    run = root / 'revisit-1-on'
    run.mkdir()
    (run / 'frames.csv').write_text(FRAME_HEADER + ''.join(
        f'{i},{start},{end},fixture,10,{settled},{moves},0,0,,0\n'
        for i, (start, end, settled, moves) in enumerate(frames)))
    (run / 'frames.render.csv').write_text('frame_id,render_schedule_wall_ms,render_thread_cpu_ms\n')
    (run / 'frames.flow.csv').write_text(FLOW_HEADER + ''.join(
        f'0,{t},{level},{waiting},0,{gen},0,0,{relight},0,0,{mesh},0,{fluid}\n'
        for t, level, gen, relight, mesh, fluid, waiting in
        ((*row, 0) if len(row) == 6 else row for row in flow)))
    PROFILE['analyze'](root)
    return json.loads((root / 'streaming.json').read_text())['revisit-1-on']['moves']


class Moves(unittest.TestCase):
    def test_each_move_settles_after_it_is_first_seen_unsettled(self):
        frames = [(21.9, 22.0, 'true', 0), (22.0, 22.1, 'true', 1), (22.1, 22.2, 'false', 1),
                  (22.2, 23.0, 'false', 1), (23.0, 23.1, 'true', 1), (23.1, 23.2, 'true', 2),
                  (23.2, 23.3, 'false', 2), (23.3, 23.5, 'true', 2)]
        with tempfile.TemporaryDirectory() as folder:
            moves = moves_run(Path(folder), frames, [(21.95, 0, 5, 1, 1, 0), (21.95, 1, 2, 0, 1, 0)])
        self.assertEqual([m['move'] for m in moves], [1, 2])
        self.assertAlmostEqual(moves[0]['settle_s'], 1.0)
        self.assertAlmostEqual(moves[1]['settle_s'], 0.2)
        self.assertAlmostEqual(moves[0]['blank_s'], 0.0)

    def test_a_move_that_never_settles_has_no_settle_time(self):
        frames = [(21.9, 22.0, 'true', 0), (22.0, 22.1, 'false', 1), (22.1, 22.2, 'false', 1)]
        with tempfile.TemporaryDirectory() as folder:
            moves = moves_run(Path(folder), frames, [])
        self.assertIsNone(moves[0]['settle_s'])

    def test_worker_cpu_until_settled_counts_only_the_move_and_its_coarse_generation_share(self):
        frames = [(21.9, 22.0, 'true', 0), (22.0, 22.1, 'false', 1), (22.1, 23.0, 'false', 1),
                  (23.0, 23.1, 'true', 1), (23.1, 25.0, 'true', 1)]
        # level, then generate, relight, mesh and fluid CPU since the World opened.
        flow = [(21.95, 0, 100, 50, 30, 7), (21.95, 1, 40, 0, 10, 7),
                (22.5, 0, 130, 60, 35, 8), (22.5, 1, 60, 0, 12, 8),
                (23.05, 0, 140, 70, 40, 9), (23.05, 1, 100, 0, 20, 9),
                (24.0, 0, 900, 900, 900, 90), (24.0, 1, 900, 0, 900, 90)]
        with tempfile.TemporaryDirectory() as folder:
            move = moves_run(Path(folder), frames, flow)[0]
        self.assertEqual(move['worker_cpu_ms']['0'], dict(generate=40, relight=20, mesh=10))
        self.assertEqual(move['worker_cpu_ms']['1'], dict(generate=60, relight=0, mesh=10))
        self.assertEqual(move['fluid_cpu_ms'], 2)
        self.assertAlmostEqual(move['coarse_generate_share'], 60 / 142)

    def test_a_reopened_world_counts_worker_cpu_from_zero(self):
        frames = [(21.9, 22.0, 'true', 0), (24.0, 24.1, 'false', 1), (24.1, 25.0, 'true', 1)]
        flow = [(21.95, 0, 1000, 500, 300, 0), (21.95, 1, 400, 0, 100, 0),
                (24.05, 0, 10, 5, 0, 0), (24.05, 1, 4, 0, 0, 0),
                (24.5, 0, 30, 10, 10, 0), (24.5, 1, 40, 0, 10, 0)]
        with tempfile.TemporaryDirectory() as folder:
            move = moves_run(Path(folder), frames, flow)[0]
        self.assertAlmostEqual(move['blank_s'], 2.0)
        self.assertAlmostEqual(move['settle_s'], 0.1)
        self.assertEqual(move['worker_cpu_ms']['1'], dict(generate=40, relight=0, mesh=10))
        self.assertAlmostEqual(move['coarse_generate_share'], 40 / 100)

    def test_coarse_generation_finishes_when_no_coarse_level_waits(self):
        frames = [(21.9, 22.0, 'true', 0), (22.0, 22.1, 'false', 1), (22.1, 23.0, 'true', 1),
                  (23.0, 26.0, 'true', 1)]
        # The last field is Chunks of the level's load extent waiting to generate.
        flow = [(21.95, 0, 0, 0, 0, 0, 0), (21.95, 1, 0, 0, 0, 0, 0),
                (22.05, 0, 10, 0, 0, 0, 900), (22.05, 1, 0, 0, 0, 0, 900),
                (23.5, 0, 20, 10, 10, 0, 0), (23.5, 1, 30, 0, 5, 0, 4),
                (24.5, 0, 20, 10, 10, 0, 0), (24.5, 1, 80, 0, 10, 0, 0),
                (25.5, 0, 20, 10, 10, 0, 0), (25.5, 1, 90, 0, 20, 0, 0)]
        with tempfile.TemporaryDirectory() as folder:
            move = moves_run(Path(folder), frames, flow)[0]
        coarse = move['coarse_generated']
        self.assertAlmostEqual(coarse['s'], 2.5)
        self.assertEqual(coarse['worker_cpu_ms']['1'], dict(generate=80, relight=0, mesh=10))
        self.assertAlmostEqual(coarse['coarse_generate_share'], 80 / 130)

    def test_coarse_generation_unfinished_within_its_move_has_no_time(self):
        frames = [(21.9, 22.0, 'true', 0), (22.0, 22.1, 'false', 1), (22.1, 23.0, 'true', 1),
                  (23.0, 23.1, 'true', 2)]
        flow = [(22.05, 1, 0, 0, 0, 0, 900), (22.9, 1, 0, 0, 0, 0, 5), (23.05, 1, 0, 0, 0, 0, 0)]
        with tempfile.TemporaryDirectory() as folder:
            move = moves_run(Path(folder), frames, flow)[0]
        self.assertIsNone(move['coarse_generated'])


if __name__ == '__main__':
    unittest.main()
