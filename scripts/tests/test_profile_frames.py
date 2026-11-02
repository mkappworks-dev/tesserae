"""Run with python3 -m unittest discover -s scripts/tests."""
import json
from pathlib import Path
import runpy
import tempfile
import unittest
from unittest.mock import patch, Mock
import subprocess

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


class WaitAnalysis(unittest.TestCase):
    def test_intervals_join_by_frame_not_row_order_and_keep_missing_values(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            run = root / 'flight-1-on'
            run.mkdir()
            (run / 'frames.csv').write_text(
                'frame_id,phase,frame_ms\n0,startup,10\n1,fixture,20\n2,fixture,30\n')
            (run / 'frames.render.csv').write_text(
                'frame_id,render_schedule_wall_ms,render_thread_cpu_ms,window_prepare_interval_ms\n'
                '2,25,2,\n0,9,1,7\n1,19,2,16\n99,99,99,99\n')
            PROFILE['analyze'](root)
            result = json.loads((root / 'summary.json').read_text())['flight-1-on']
            self.assertEqual(result['fixture']['window_prepare_interval_ms'],
                             dict(n=1, p50=16, p95=16, p99=16, max=16))
            self.assertEqual(result['startup']['window_prepare_interval_ms']['p50'], 7)

    def test_monitor_confirmation_rejects_wrong_missing_or_mixed_displays(self):
        with tempfile.TemporaryDirectory() as folder:
            run = Path(folder)
            log = run / 'native.log'
            for text, expected in [('', False),
                                   ('profile_monitor_selected refresh_millihertz=120000', False),
                                   ('profile_monitor_selected refresh_millihertz=143970', True),
                                   ('profile_monitor_selected refresh_millihertz=144000\n'
                                    'profile_monitor_selected refresh_millihertz=120000', False)]:
                log.write_text(text)
                self.assertEqual(PROFILE['monitor_selected'](run, 144), expected)
            log.write_text('profile_monitor_selected refresh_millihertz=144000')
            (run / 'frames.metadata.txt').write_text('reported_refresh_millihertz=Some(120000)')
            self.assertFalse(PROFILE['monitor_selected'](run, 144))
            (run / 'frames.metadata.txt').write_text(
                'reported_refresh_millihertz=Some(144000)\nwindow=1280x720 scale=1')
            self.assertFalse(PROFILE['monitor_selected'](run, 144, (2560, 1440)))
            (run / 'frames.metadata.txt').write_text(
                'reported_refresh_millihertz=Some(144000)\nwindow=2560x1440 scale=1')
            self.assertTrue(PROFILE['monitor_selected'](run, 144, (2560, 1440)))

    def test_clipped_size_reports_actual_dimensions_and_borderless_needs_confirmation(self):
        with tempfile.TemporaryDirectory() as folder:
            run = Path(folder)
            (run / 'native.log').write_text(
                'profile_monitor_selected refresh_millihertz=144000 borderless=true')
            meta = run / 'frames.metadata.txt'
            meta.write_text('reported_refresh_millihertz=Some(144000)\n'
                            'window=2560x1378 scale=1 mode=Windowed')
            error = PROFILE['monitor_selection_error'](run, 144, (2560, 1440), True)
            self.assertIn('actual content size is 2560x1378', error)
            meta.write_text('reported_refresh_millihertz=Some(144000)\n'
                            'window=2560x1440 scale=1 mode=Windowed')
            self.assertIn('not borderless',
                          PROFILE['monitor_selection_error'](run, 144, (2560, 1440), True))
            meta.write_text('reported_refresh_millihertz=Some(144000)\n'
                            'window=2560x1440 scale=1 mode=BorderlessFullscreen(Entity(1v0))')
            self.assertIsNone(PROFILE['monitor_selection_error'](run, 144, (2560, 1440), True))
            (run / 'native.log').write_text('profile_monitor_selected refresh_millihertz=144000')
            self.assertIn('did not confirm the borderless',
                          PROFILE['monitor_selection_error'](run, 144, (2560, 1440), True))

    def test_host_query_failure_is_retained_without_losing_other_observations(self):
        with patch('subprocess.run', side_effect=[
                subprocess.TimeoutExpired('pmset', 5),
                subprocess.CompletedProcess(['pmset'], 1, '', 'not supported')]):
            result = PROFILE['host_observation']()
        self.assertIn('error', result['power'])
        self.assertEqual(result['thermal']['returncode'], 1)
        self.assertEqual(result['thermal']['stderr'], 'not supported')
        self.assertEqual(len(result['load_average']), 3)


class MonitorProbe(unittest.TestCase):
    def test_probe_runs_once_without_requiring_or_analyzing_fixture_samples(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            engine = root / 'engine'
            for name in ['Cargo.toml', 'Cargo.lock', 'crates/tesserae/Cargo.toml']:
                target = engine / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text('test manifest')
            game = root / 'game/mods'
            game.mkdir(parents=True)
            binary = root / 'test-binary'
            binary.write_text('test executable')
            output = root / 'probe'
            analyze = Mock()
            def process(*args, **kwargs):
                run = output / 'stationary-1-on'
                (run / 'native.log').write_text(
                    'profile_monitor_selected refresh_millihertz=144000 borderless=true')
                (run / 'frames.metadata.txt').write_text(
                    'reported_refresh_millihertz=Some(144000)\n'
                    'window=2560x1440 mode=BorderlessFullscreen(Entity(1v0))')
                return subprocess.CompletedProcess(args[0], 0)
            def local_path(value):
                return root / 'lock' if str(value) == '/tmp/tesserae-frame-profile.lock' else Path(value)
            argv = ['profile-frames', str(output), '--binary', str(binary),
                    '--profile', 'release', '--features', 'profiling', '--game', str(game),
                    '--presented', '--monitor-hz', '144', '--borderless', '--size', '2560x1440',
                    '--monitor-probe']
            with patch.dict(PROFILE['main'].__globals__, {
                    'ROOT': engine, 'Path': local_path, 'analyze': analyze,
                    'competitors': lambda: [], 'host_observation': lambda: {'load_average': [0, 0, 0]}}):
                with patch('sys.argv', argv), patch('subprocess.run', side_effect=process) as calls:
                    with patch('subprocess.check_output', side_effect=lambda *a, **kw: 'test' if kw.get('text') else b''):
                        PROFILE['main']()
            self.assertEqual(calls.call_count, 1)
            analyze.assert_not_called()
            env = calls.call_args.kwargs['env']
            self.assertEqual(env['TESSERAE_PROFILE_MONITOR_PROBE'], '1')
            self.assertEqual(env['TESSERAE_PROFILE_BORDERLESS'], '1')
            self.assertFalse((root / 'lock').exists())
            self.assertFalse((output / 'stationary-1-on/frames.csv').exists())


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
