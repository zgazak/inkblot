"""Run all example scripts and report results."""
import subprocess
import sys
import time
from pathlib import Path

examples_dir = Path(__file__).parent
scripts = sorted(examples_dir.glob("*.py"))
scripts = [s for s in scripts if s.name != "run_all.py"]

print(f"Running {len(scripts)} examples...\n")

failed = []
for script in scripts:
    name = script.stem
    t0 = time.perf_counter()
    result = subprocess.run(
        [sys.executable, str(script)],
        capture_output=True, text=True, cwd=examples_dir.parent,
    )
    dt = time.perf_counter() - t0
    if result.returncode == 0:
        print(f"  {name:.<30s} ok  ({dt:.3f}s)")
    else:
        print(f"  {name:.<30s} FAIL ({dt:.3f}s)")
        print(f"    {result.stderr.strip()}")
        failed.append(name)

print()
if failed:
    print(f"{len(failed)} failed: {', '.join(failed)}")
    sys.exit(1)
else:
    print(f"All {len(scripts)} examples passed.")
