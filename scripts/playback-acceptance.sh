#!/usr/bin/env bash
# Source-only launcher: run explicitly after the parent releases the build lane.
set -euo pipefail
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
exec python3 - "$repo_root" "$@" <<'PY'
import argparse
import fcntl
import hashlib
import ipaddress
import json
import os
from pathlib import Path
from fractions import Fraction
import re
import shlex
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.parse
import urllib.request


class Failure(Exception):
    pass


def require(condition, message):
    if not condition:
        raise Failure(message)


def say(message):
    print(message, flush=True)


repo = Path(sys.argv[1])
parser = argparse.ArgumentParser(
    prog=str(repo / "scripts/playback-acceptance.sh"),
    description="Disposable real-media acceptance; requires an idle, existing cgroup v2 build lane.",
)
parser.add_argument("build_cgroup", type=Path)
parser.add_argument("run_directory", type=Path, help="absolute, nonexistent path with an existing parent")
parser.add_argument("--serve", action="store_true", help="hold for manual use instead of running Chromium")
parser.add_argument("--lan-ip", metavar="IPV4", help="with --serve only: bind an assigned RFC1918 IPv4 address; no automatic discovery or casting")
parser.add_argument("--build-timeout", type=int, default=7200, metavar="SECONDS")
parser.add_argument("--ready-timeout", type=int, default=300, metavar="SECONDS")
parser.add_argument("--browser-timeout", type=int, default=900, metavar="SECONDS")
args = parser.parse_args(sys.argv[2:])
interrupted = 0


def on_signal(number, _frame):
    global interrupted
    interrupted = number


for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
    signal.signal(sig, on_signal)


def check_signal():
    if interrupted:
        raise Failure(f"Interrupted by signal {interrupted}")


class OwnedProcess:
    """Keep the session leader unreaped until its group has been cleaned up.

    WNOWAIT reserves the leader PID even after exit, preventing a reused PID/PGID
    from ever becoming a cleanup target. Never signal names, ports or cgroups.
    """

    def __init__(self, command, cwd, env, log, stdout=None):
        check_signal()
        with log.open("wb") as errors:
            if stdout is None:
                self.process = subprocess.Popen(command, cwd=cwd, env=env,
                    stdin=subprocess.DEVNULL, stdout=errors, stderr=subprocess.STDOUT,
                    start_new_session=True)
            else:
                with stdout.open("wb") as output:
                    self.process = subprocess.Popen(command, cwd=cwd, env=env,
                        stdin=subprocess.DEVNULL, stdout=output, stderr=errors,
                        start_new_session=True)
        self.closed = False

    def status(self):
        result = os.waitid(os.P_PID, self.process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        if result is None:
            return None
        return result.si_status if result.si_code == os.CLD_EXITED else -result.si_status

    def signal_group(self, sig):
        try:
            os.killpg(self.process.pid, sig)
        except ProcessLookupError:
            pass

    def close(self):
        if self.closed:
            return
        self.signal_group(signal.SIGTERM)
        deadline = time.monotonic() + 10
        while self.status() is None and time.monotonic() < deadline:
            time.sleep(0.1)
        # Also remove children left behind by an exited leader, before reaping it.
        self.signal_group(signal.SIGKILL)
        self.process.wait()
        self.closed = True


def wait_process(child, seconds, label, companions=()):
    deadline = time.monotonic() + seconds
    while True:
        check_signal()
        for other in companions:
            require(other.status() is None, f"A server exited during {label}; inspect logs")
        status = child.status()
        if status is not None:
            return status
        require(time.monotonic() < deadline, f"Timed out after {seconds}s: {label}")
        time.sleep(0.2)


def parse_cpu_set(cpulist):
    cpus = set()
    for part in cpulist.strip().split(","):
        first, _, last = part.partition("-")
        cpus.update(range(int(first), int(last or first) + 1))
    return cpus


def cgroup_contract(group):
    # Resolve the real cgroup2 mount, including a namespace's non-root mount root.
    mounts = []
    unescape = lambda value: re.sub(r"\\([0-7]{3})", lambda m: chr(int(m[1], 8)), value)
    for line in Path("/proc/self/mountinfo").read_text().splitlines():
        before, after = line.split(" - ", 1)
        if after.split()[0] == "cgroup2":
            fields = before.split()
            mount = Path(unescape(fields[4])).resolve()
            if group.is_relative_to(mount):
                mounts.append((mount, Path(unescape(fields[3]))))
    require(mounts, "Build group must be inside a mounted cgroup v2 filesystem")
    mount, mount_root = max(mounts, key=lambda item: len(item[0].parts))
    own = next((line[3:] for line in Path("/proc/self/cgroup").read_text().splitlines()
                if line.startswith("0::")), None)
    require(own is not None and Path(own).is_relative_to(mount_root),
            "Cannot establish the launcher's cgroup v2 ancestry")
    caller_group = mount / Path(own).relative_to(mount_root)
    require(caller_group.is_dir(), "Cannot inspect the launcher's cgroup")
    require(group != mount and not caller_group.is_relative_to(group),
            "Launcher and application servers must start outside the build cgroup")
    require(not any(path.is_dir() for path in group.iterdir()),
            "Use a dedicated leaf build cgroup with no child cgroups")
    require(os.access(group / "cgroup.procs", os.W_OK), "Build cgroup.procs must be writable")
    require((group / "cgroup.type").read_text().strip() == "domain",
            "Build group must be a domain cgroup")
    available = Fraction(len(os.sched_getaffinity(0)))
    cpuset = group / "cpuset.cpus.effective"
    if cpuset.exists() and cpuset.read_text().strip():
        available = min(available, Fraction(len(parse_cpu_set(cpuset.read_text()) & os.sched_getaffinity(0))))
    # Container/parent quotas may be lower than nproc/affinity. Exclude the build
    # leaf's own quota when calculating the capacity to which its 80% cap applies.
    for start in (caller_group, group.parent):
        for ancestor in (start, *start.parents):
            if not ancestor.is_relative_to(mount):
                break
            limit = ancestor / "cpu.max"
            if limit.exists():
                quota, period = limit.read_text().split()
                if quota != "max":
                    available = min(available, Fraction(int(quota), int(period)))
    quota, period = (group / "cpu.max").read_text().split()
    require(quota != "max" and int(quota) > 0 and int(period) > 0,
            "Build cpu.max must specify a finite positive aggregate quota")
    actual = Fraction(int(quota), int(period))
    require(available > 0 and actual <= available * Fraction(4, 5),
            f"Build quota {float(actual):g} CPUs exceeds 80% of {float(available):g} available CPUs")
    return {"cpuMax": f"{quota} {period}", "availableCpus": float(available),
            "buildCgroup": str(group), "launcherCgroup": str(caller_group)}


def idle(group):
    events = dict(line.split() for line in (group / "cgroup.events").read_text().splitlines())
    require(events.get("populated") == "0" and not (group / "cgroup.procs").read_text().strip(),
            f"Refusing active build cgroup {group}; ask the parent to release the lane and use a new run path")


def lan_address(value):
    require(args.serve, "--lan-ip requires --serve; automated browser acceptance stays on loopback")
    try:
        address = ipaddress.IPv4Address(value)
    except ipaddress.AddressValueError as error:
        raise Failure("--lan-ip must be an RFC1918 local IPv4 literal, without a port or prefix") from error
    require(any(address in ipaddress.IPv4Network(network)
                for network in ("10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16")),
            "--lan-ip must be RFC1918; wildcard, loopback, link-local and public addresses are refused")
    ip_tool = shutil.which("ip")
    require(ip_tool, "LAN serving requires iproute2's ip command to verify local address assignment")
    # Local interface inspection only: no DNS, discovery, connection or route probe.
    result = subprocess.run([ip_tool, "-j", "-4", "address", "show"], check=False,
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, timeout=10)
    require(result.returncode == 0, "Cannot inspect assigned IPv4 addresses with iproute2")
    interfaces = json.loads(result.stdout)
    require(isinstance(interfaces, list), "Unexpected iproute2 address output")
    require(any(str(address) == entry.get("local") and entry.get("family") == "inet"
                for interface in interfaces if "UP" in interface.get("flags", [])
                for entry in interface.get("addr_info", [])),
            f"--lan-ip {address} is not assigned to an UP interface on this host; foreign hosts are refused")
    return str(address)


def free_ui_port(host):
    with socket.socket() as probe:
        # Match Node's reusable listener: closed accepted sockets can remain in
        # TIME_WAIT after a clean run without any process still owning the port.
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            probe.bind((host, 3003))
        except OSError as error:
            raise Failure(f"Port 3003 is occupied; leave its owner alone: {error}") from error


def read_ready(path, server, seconds, temp, host):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        check_signal()
        require(server.status() is None, "Fixture exited before readiness; inspect logs/fixture.log")
        try:
            manifest = json.loads(path.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(0.2)  # Fixture writes JSON to a create_new file, not by rename.
            continue
        url = urllib.parse.urlsplit(manifest.get("backendUrl", ""))
        require(manifest.get("version") == 1 and url.scheme == "http"
                and url.hostname == host and url.port and url.path in ("", "/")
                and not url.username and not url.password and not url.query and not url.fragment,
                "Unexpected disposable ready.json backend/version contract")
        require(Path(manifest["fixtureRoot"]).resolve().is_relative_to(temp),
                "Fixture root must be inside this run's TMPDIR")
        require(manifest.get("credentials", {}).get("email") and manifest["credentials"].get("password"),
                "Fixture readiness lacks login credentials")
        return manifest
    raise Failure(f"Fixture did not produce valid ready.json within {seconds}s")


children = []
fixture = None
run = None
lock_fd = None
exit_code = 1
cleanup_failed = False
try:
    require(sys.platform == "linux" and sys.version_info >= (3, 9), "Requires Linux and Python 3.9+")
    host = lan_address(args.lan_ip) if args.lan_ip is not None else "127.0.0.1"
    ui_url = f"http://{host}:3003"
    require(all(value > 0 for value in (args.build_timeout, args.ready_timeout, args.browser_timeout)),
            "Timeouts must be positive seconds")
    group = args.build_cgroup.resolve(strict=True)
    require(group.is_dir(), "Build cgroup must already exist")
    resources = cgroup_contract(group)
    # The directory inode is the shared advisory lock, independent of run paths.
    # Other users of this lane must honor this lock/exclusive scheduling contract.
    lock_fd = os.open(group, os.O_RDONLY | os.O_DIRECTORY)
    try:
        fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as error:
        raise Failure("Another playback launcher owns this build lane") from error
    idle(group)
    free_ui_port(host)
    tools = {name: shutil.which(name) for name in ("bash", "cargo", "node", "ffmpeg", "ffprobe")}
    require(all(tools.values()), "Install bash, Cargo, Node, FFmpeg and FFprobe before running")
    require((repo / "backend/Cargo.lock").is_file(), "Requires the existing backend Cargo.lock")
    require((repo / "web/node_modules/vite/package.json").is_file(),
            "Requires already-installed web dependencies; no automatic installs")
    if not args.serve:
        require((repo / "web/node_modules/@playwright/test/package.json").is_file(),
                "Requires already-installed Playwright Chromium; no automatic installs")
    # Do not inherit tooling that stubs runtime code. Keep normal parent profile,
    # allocator, target-dir and compiler settings unchanged for cache compatibility.
    for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
        value = os.environ.get(key, "")
        require(not any(marker in value for marker in ("schema-export", "schema_export", "rehearsal")),
                f"Refusing runtime-altering tooling in {key}")
    requested = args.run_directory
    require(requested.is_absolute() and not os.path.lexists(requested),
            "Run directory must be absolute and nonexistent; never reuse a previous run")
    os.umask(0o077)
    requested.mkdir(mode=0o700)  # Exclusive reservation; do not mkdir -p or follow an existing target.
    run = requested.resolve(strict=True)
    for name in ("logs", "bin", "tmp"):
        (run / name).mkdir()
    resources["testProfileEnvironment"] = {key: value for key, value in os.environ.items()
        if key.startswith("CARGO_PROFILE_TEST_") or key in ("CARGO_INCREMENTAL", "CARGO_TARGET_DIR")}
    (run / "resources.json").write_text(json.dumps(resources, indent=2) + "\n")
    say(f"Acceptance run: {run}\nBuild log: {run / 'logs/build.log'}")

    # Both the Cargo exec and every real encoder join the SAME capped leaf.
    # BASHPID is the wrapper process, not the outside supervisor/test server.
    wrapper = run / "bin/ffmpeg"
    wrapper.write_text("#!/usr/bin/env bash\nset -euo pipefail\n"
        + f"printf '%s\\n' \"$BASHPID\" > {shlex.quote(str(group / 'cgroup.procs'))}\n"
        + f"exec {shlex.quote(tools['ffmpeg'])} \"$@\"\n")
    wrapper.chmod(0o700)
    build_env = dict(os.environ, SQLX_OFFLINE="true")
    command = [tools["cargo"], "test", "--locked", "--offline", "--no-default-features",
        "--features", "sqlite", "--test", "playback_acceptance", "--no-run",
        "--message-format=json-render-diagnostics", "-j", "1"]
    (run / "build-command.json").write_text(json.dumps(command, indent=2) + "\n")
    idle(group)  # Refuse a newly occupied lane before admitting Cargo.
    join = f'printf \'%s\\n\' "$BASHPID" > {shlex.quote(str(group / "cgroup.procs"))}; exec "$@"'
    build = OwnedProcess([tools["bash"], "-euc", join, "playback-build", *command],
        repo / "backend", build_env, run / "logs/build.log", run / "logs/cargo.jsonl")
    children.append(build)
    require(wait_process(build, args.build_timeout, "Cargo no-run build") == 0,
            "Cargo compilation failed; inspect logs/build.log and logs/cargo.jsonl")
    build.close()
    idle(group)
    executables = set()
    finished = False
    with (run / "logs/cargo.jsonl").open() as output:
        for line in output:
            try:
                message = json.loads(line)
            except json.JSONDecodeError as error:
                raise Failure("Cargo stdout contained non-JSON output; inspect logs/cargo.jsonl") from error
            if message.get("reason") == "build-finished":
                finished = message.get("success") is True
            if (message.get("reason") == "compiler-artifact"
                    and message.get("target", {}).get("name") == "playback_acceptance"
                    and "test" in message["target"].get("kind", [])
                    and message.get("profile", {}).get("test") is True
                    and message.get("executable")):
                features = message.get("features", [])
                require("sqlite" in features and "schema-export-only" not in features,
                        "Cargo artifact does not qualify the normal SQLite runtime")
                require(Path(message["manifest_path"]).resolve() == (repo / "backend/Cargo.toml").resolve(),
                        "Cargo artifact belongs to an unexpected manifest")
                executables.add(message["executable"])
    require(finished and len(executables) == 1, "Expected exactly one successful playback_acceptance test artifact")
    binary = run / "bin/playback_acceptance"
    shutil.copy2(Path(executables.pop()), binary)
    digest = hashlib.sha256()
    with binary.open("rb") as copied:
        for chunk in iter(lambda: copied.read(1024 * 1024), b""):
            digest.update(chunk)
    (run / "binary-sha256.txt").write_text(digest.hexdigest() + "\n")

    # Snapshot UI inputs: router generation, Vite config output and caches stay
    # disposable. Installed dependencies are shared read-only inputs to this run.
    web = run / "web"
    web.mkdir()
    for name in ("src", "public"):
        shutil.copytree(repo / "web" / name, web / name)
    for name in ("index.html", "package.json", "tsconfig.json", "tsconfig.build.json", "vite.config.ts"):
        shutil.copy2(repo / "web" / name, web / name)
    (web / "node_modules").symlink_to((repo / "web/node_modules").resolve(), target_is_directory=True)
    (web / "e2e").mkdir()
    shutil.copy2(repo / "web/e2e/playback-browser.mjs", web / "e2e/playback-browser.mjs")
    ui_build_entry = web / "acceptance-build.mjs"
    ui_build_entry.write_text('''import { build } from 'vite';
await build({ configFile: './vite.config.ts', configLoader: 'runner',
  cacheDir: './.vite-cache', clearScreen: false });
''')
    vite_entry = web / "acceptance-vite.mjs"
    vite_entry.write_text('''import { preview } from 'vite';
import { writeFileSync } from 'node:fs';
const server = await preview({
  configFile: './vite.config.ts', configLoader: 'runner',
  cacheDir: './.vite-cache', clearScreen: false,
  preview: {
    host: process.env.PLAYBACK_BIND_IP, port: 3003, strictPort: true, open: false,
    ...(process.env.PLAYBACK_BIND_IP !== '127.0.0.1' ? {
      allowedHosts: [process.env.PLAYBACK_BIND_IP],
      cors: { origin: `http://${process.env.PLAYBACK_BIND_IP}:3003` },
    } : {}),
  },
});
writeFileSync(process.env.PLAYBACK_VITE_READY, JSON.stringify({ port: 3003 }));
let closing = false;
for (const sig of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(sig, async () => {
  if (closing) return;
  closing = true;
  await server.close();
  process.exit(0);
});
''')
    # Runtime processes cannot inherit DATABASE_URL, production configuration,
    # alternate backend/public origins or an accidental browser case filter.
    runtime_env = {key: os.environ[key] for key in ("PATH", "HOME", "LANG", "TZ", "LIBRARIAN_TEST_LOG") if key in os.environ}
    runtime_env["PATH"] = str(run / "bin") + os.pathsep + os.environ.get("PATH", os.defpath)
    runtime_env.update(TMPDIR=str(run / "tmp"), LIBRARIAN_ACCEPTANCE_FFMPEG=str(wrapper),
        LIBRARIAN_PLAYBACK_ACCEPTANCE_DIR=str(run))
    if args.lan_ip is not None:
        # Never inherit the opt-in from a shell; only the validated explicit flag.
        runtime_env["LIBRARIAN_PLAYBACK_LAN_IP"] = host
    fixture = OwnedProcess([str(binary), "--exact", "real_media_fixture_and_stream_contract",
        "--nocapture", "--test-threads=1"], run, runtime_env, run / "logs/fixture.log")
    children.append(fixture)
    manifest = read_ready(run / "ready.json", fixture, args.ready_timeout, run / "tmp", host)
    say(f"Fixture ready: {run / 'ready.json'}")
    free_ui_port(host)
    vite_env = dict(runtime_env, PORT="3003", BACKEND_PROXY_TARGET=manifest["backendUrl"],
        DEV_SERVER_PUBLIC_URL="", PLAYBACK_BIND_IP=host, PLAYBACK_VITE_READY=str(run / "vite-ready.json"))
    # Build the copied current sources under the same aggregate CPU quota.
    # Serve production assets to avoid development dependency discovery/HMR
    # invalidating a browser check while a newly visited route initializes.
    ui_build = OwnedProcess([tools["bash"], "-euc", join, "playback-ui-build",
        tools["node"], str(ui_build_entry)], web, vite_env, run / "logs/ui-build.log")
    children.append(ui_build)
    require(wait_process(ui_build, args.build_timeout, "production UI build", (fixture,)) == 0,
            "Production UI build failed; inspect logs/ui-build.log")
    ui_build.close()
    vite = OwnedProcess([tools["node"], str(vite_entry)], web, vite_env, run / "logs/vite.log")
    children.append(vite)
    deadline = time.monotonic() + args.ready_timeout
    while not (run / "vite-ready.json").exists():
        check_signal()
        require(fixture.status() is None and vite.status() is None, "Server startup failed; inspect fixture/vite logs")
        require(time.monotonic() < deadline, "Vite startup timed out; inspect logs/vite.log")
        time.sleep(0.2)
    # Verify the owned server's proxy before handing off. No DB or SQL access.
    request = urllib.request.Request(f"{ui_url}/graphql",
        data=json.dumps({"query": "query { __typename }"}).encode(),
        headers={"Content-Type": "application/json", "Origin": ui_url})
    with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request, timeout=10) as response:
        require(json.load(response).get("data", {}).get("__typename") == "Query",
                "Isolated Vite GraphQL proxy did not reach the fixture")
    if args.serve:
        say(f"Manual UI: {ui_url}\nCredentials: {run / 'ready.json'}\n"
            f"Stop: Ctrl-C or create {run / 'stop'} (fixture maximum: two hours after readiness).")
        if args.lan_ip is not None:
            say(f"LAN media backend: {manifest['backendUrl']}\n"
                "Disposable per-run credentials; plain HTTP without TLS on the local test LAN.\n"
                "Cast service ready with auto-discovery disabled; no discovery or receiver LOAD was requested.")
        while fixture.status() is None:
            check_signal()
            require(vite.status() is None, "Vite exited during manual hold")
            time.sleep(0.25)
        require(fixture.status() == 0, "Fixture failed during manual hold; inspect logs/fixture.log")
    else:
        say(f"Running real-media Chromium acceptance; log: {run / 'logs/browser.log'}")
        browser_env = dict(runtime_env, PLAYBACK_UI_URL="http://127.0.0.1:3003")
        # Browser cache location may be needed when Chromium was preinstalled
        # somewhere other than HOME/.cache. No other runner options are inherited.
        if "PLAYWRIGHT_BROWSERS_PATH" in os.environ:
            browser_env["PLAYWRIGHT_BROWSERS_PATH"] = os.environ["PLAYWRIGHT_BROWSERS_PATH"]
        browser = OwnedProcess([tools["node"], str(web / "e2e/playback-browser.mjs"), str(run / "ready.json")],
            web, browser_env, run / "logs/browser.log")
        children.append(browser)
        require(wait_process(browser, args.browser_timeout, "browser acceptance", (fixture, vite)) == 0,
                "Browser acceptance failed; inspect logs/browser.log and browser/results.json")
        browser.close()
        results = json.loads((run / "browser/results.json").read_text()).get("results", [])
        require(results and all(row.get("passed") is True for row in results),
                "Browser results are empty or contain failures")
        say(f"Browser checks passed: {len(results)}; evidence: {run / 'browser'}")
    exit_code = 0
except (Failure, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
    say(f"Acceptance failed: {error}")
finally:
    # SIGINT/TERM/HUP are handled here as well as normal/error exits. Ask the
    # fixture to shut down first; only escalate within sessions we created.
    if fixture is not None and not fixture.closed:
        try:
            (run / "stop").touch(exist_ok=True)
            deadline = time.monotonic() + 30
            while fixture.status() is None and time.monotonic() < deadline:
                time.sleep(0.2)
            if fixture.status() != 0:
                cleanup_failed = True
                say("Fixture did not shut down successfully; inspect logs/fixture.log")
        except OSError as error:
            cleanup_failed = True
            say(f"Could not request fixture shutdown: {error}")
    for child in reversed(children):
        try:
            child.close()
        except OSError as error:
            cleanup_failed = True
            say(f"Owned-process cleanup failed: {error}")
    if lock_fd is not None:
        os.close(lock_fd)
    if run is not None:
        say(f"Stopped owned processes; retained run artifacts: {run}")
sys.exit(128 + interrupted if interrupted else (1 if cleanup_failed else exit_code))
PY
