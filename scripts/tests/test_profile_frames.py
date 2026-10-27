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


if __name__ == '__main__':
    unittest.main()
