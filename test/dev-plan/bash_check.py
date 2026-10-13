import shutil
import subprocess
from pathlib import Path


def run_progress(script, plan, root, timeout):
    bash = shutil.which("bash")
    result = subprocess.run(
        [bash, Path(script).as_posix(), Path(plan).as_posix(), Path(root).as_posix()],
        capture_output=True, timeout=timeout,
    )
    try:
        result.stdout = result.stdout.decode("utf-8")
        result.stderr = result.stderr.decode("utf-8")
    except UnicodeDecodeError as error:
        raise AssertionError(
            f"Bash {bash!r} returned non-UTF-8 output: "
            f"stdout={result.stdout!r}; stderr={result.stderr!r}"
        ) from error
    return result
