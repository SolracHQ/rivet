#!/usr/bin/env python3
"""
Simple development environment manager for Rivet.

Usage:
    ./dev.py start   - Build and start all services
    ./dev.py stop    - Stop all services
    ./dev.py restart - Restart all services
    ./dev.py logs    - Tail all log files
    ./dev.py clean   - Stop services and remove data
"""

import os
import signal
import subprocess
import sys
import time
from pathlib import Path

PROJECT_DIR = Path(__file__).parent
LOGS_DIR = PROJECT_DIR / "logs"
PID_FILE = LOGS_DIR / "services.pid"

POSTGRES_CONTAINER = "rivet_postgres"
POSTGRES_PORT = 5432
POSTGRES_USER = "rivet"
POSTGRES_PASSWORD = "rivet"
POSTGRES_DB = "rivet"


def ensure_logs_dir():
    """Create logs directory if it doesn't exist."""
    LOGS_DIR.mkdir(exist_ok=True)


def run_command(cmd, check=True, capture_output=False):
    """Run a shell command."""
    if capture_output:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        return result.returncode == 0, result.stdout.strip()
    else:
        result = subprocess.run(cmd, shell=True, check=check)
        return result.returncode == 0, ""


def is_postgres_running():
    """Check if postgres container is running."""
    success, output = run_command(
        f"podman ps --filter name={POSTGRES_CONTAINER} --format '{{{{.Names}}}}'",
        capture_output=True,
    )
    return success and POSTGRES_CONTAINER in output


def start_postgres():
    """Start postgres container with podman."""
    print("Starting PostgreSQL...")

    if is_postgres_running():
        print("PostgreSQL is already running")
        return True

    # Check if container exists but is stopped
    success, output = run_command(
        f"podman ps -a --filter name={POSTGRES_CONTAINER} --format '{{{{.Names}}}}'",
        capture_output=True,
    )

    if POSTGRES_CONTAINER in output:
        print("Starting existing PostgreSQL container...")
        run_command(f"podman start {POSTGRES_CONTAINER}")
    else:
        print("Creating new PostgreSQL container...")
        run_command(
            f"podman run -d --name {POSTGRES_CONTAINER} "
            f"-p {POSTGRES_PORT}:{POSTGRES_PORT} "
            f"-e POSTGRES_USER={POSTGRES_USER} "
            f"-e POSTGRES_PASSWORD={POSTGRES_PASSWORD} "
            f"-e POSTGRES_DB={POSTGRES_DB} "
            f"docker.io/postgres:16-alpine"
        )

    # Wait for postgres to be ready
    print("Waiting for PostgreSQL to be ready...")
    for i in range(30):
        success, _ = run_command(
            f"podman exec {POSTGRES_CONTAINER} pg_isready -U {POSTGRES_USER}",
            check=False,
            capture_output=True,
        )
        if success:
            print("PostgreSQL is ready")
            return True
        time.sleep(1)

    print("ERROR: PostgreSQL failed to start")
    return False


def stop_postgres():
    """Stop postgres container."""
    if is_postgres_running():
        print("Stopping PostgreSQL...")
        run_command(f"podman stop {POSTGRES_CONTAINER}", check=False)


def remove_postgres():
    """Remove postgres container and data."""
    print("Removing PostgreSQL container...")
    run_command(f"podman rm -f {POSTGRES_CONTAINER}", check=False)


def build_project():
    """Build the Rust project."""
    print("Building project...")
    os.chdir(PROJECT_DIR)
    success, _ = run_command("cargo build")
    return success


def start_orchestrator():
    """Start orchestrator in background."""
    print("Starting orchestrator...")

    log_file = LOGS_DIR / "orchestrator.log"
    pid_file = LOGS_DIR / "orchestrator.pid"

    with open(log_file, "w") as log:
        env = os.environ.copy()
        env["RUST_LOG"] = "debug"
        env["DATABASE_URL"] = (
            f"postgres://{POSTGRES_USER}:{POSTGRES_PASSWORD}@localhost:{POSTGRES_PORT}/{POSTGRES_DB}"
        )
        env["ORCHESTRATOR_BIND_ADDR"] = "0.0.0.0:8080"

        proc = subprocess.Popen(
            ["./target/debug/rivet-orchestrator"],
            stdout=log,
            stderr=subprocess.STDOUT,
            env=env,
            cwd=PROJECT_DIR,
        )

        with open(pid_file, "w") as pf:
            pf.write(str(proc.pid))

        print(f"Orchestrator started (PID: {proc.pid})")

        # Wait a bit for orchestrator to start
        time.sleep(2)

        if proc.poll() is not None:
            print("ERROR: Orchestrator failed to start")
            return False

        return True


def start_runner():
    """Start runner in background."""
    print("Starting runner...")

    log_file = LOGS_DIR / "runner.log"
    pid_file = LOGS_DIR / "runner.pid"

    with open(log_file, "w") as log:
        env = os.environ.copy()
        env["RUST_LOG"] = "debug"
        env["ORCHESTRATOR_URL"] = "http://localhost:8080"
        env["RUNNER_ID"] = "runner-1"

        proc = subprocess.Popen(
            ["./target/debug/rivet-runner"],
            stdout=log,
            stderr=subprocess.STDOUT,
            env=env,
            cwd=PROJECT_DIR,
        )

        with open(pid_file, "w") as pf:
            pf.write(str(proc.pid))

        print(f"Runner started (PID: {proc.pid})")
        return True


def stop_service(name):
    """Stop a service by reading its PID file."""
    pid_file = LOGS_DIR / f"{name}.pid"

    if not pid_file.exists():
        return

    try:
        with open(pid_file) as f:
            pid = int(f.read().strip())

        print(f"Stopping {name} (PID: {pid})...")
        os.kill(pid, signal.SIGTERM)

        # Wait for process to exit
        for _ in range(10):
            try:
                os.kill(pid, 0)
                time.sleep(0.5)
            except OSError:
                break

        pid_file.unlink()
    except (ValueError, ProcessLookupError):
        pid_file.unlink()


def start():
    """Start all services."""
    ensure_logs_dir()

    if not start_postgres():
        return False

    if not build_project():
        print("ERROR: Build failed")
        return False

    if not start_orchestrator():
        stop_postgres()
        return False

    if not start_runner():
        stop_service("orchestrator")
        stop_postgres()
        return False

    print("\nAll services started successfully!")
    print(f"Logs are in: {LOGS_DIR}")
    print("\nOrchestrator: http://localhost:8080")
    print("\nUse './dev.py logs' to tail logs")
    print("Use './dev.py stop' to stop all services")
    return True


def stop():
    """Stop all services."""
    print("Stopping all services...")
    stop_service("runner")
    stop_service("orchestrator")
    stop_postgres()
    print("All services stopped")


def restart():
    """Restart all services."""
    stop()
    time.sleep(1)
    start()


def tail_logs():
    """Tail all log files."""
    ensure_logs_dir()

    log_files = list(LOGS_DIR.glob("*.log"))

    if not log_files:
        print("No log files found")
        return

    print("Tailing logs (Ctrl+C to stop)...")
    try:
        subprocess.run(["tail", "-f"] + [str(f) for f in log_files])
    except KeyboardInterrupt:
        print("\nStopped tailing logs")


def clean():
    """Stop services and remove all data."""
    print("Cleaning up...")
    stop()
    remove_postgres()
    print("Cleanup complete")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    command = sys.argv[1]

    commands = {
        "start": start,
        "stop": stop,
        "restart": restart,
        "logs": tail_logs,
        "clean": clean,
    }

    if command not in commands:
        print(f"Unknown command: {command}")
        print(__doc__)
        sys.exit(1)

    try:
        result = commands[command]()
        if result is False:
            sys.exit(1)
    except KeyboardInterrupt:
        print("\nInterrupted")
        sys.exit(1)
    except Exception as e:
        print(f"ERROR: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
