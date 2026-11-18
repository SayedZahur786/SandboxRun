#!/usr/bin/env python3
"""Rebuild the zero-preopen CPython WASI artifact from pinned release objects.
Requires macOS arm64 (the pinned host tools) and Python 3.12+. No guest source
is executed on the build host. Read runtimes/python.provenance.json for inputs.
"""
from pathlib import Path
import hashlib, json, platform, shutil, subprocess, tarfile, urllib.request, zipfile

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / 'runtimes/python.provenance.json').read_text())
if platform.system() != 'Darwin' or platform.machine() != 'arm64':
    raise SystemExit('Pinned rebuild toolchain requires macOS arm64; the resulting WASM is portable.')
work = root / 'work/python-build-repro'
work.mkdir(parents=True, exist_ok=True)
names = ['build.zip', 'python.zip', 'vfs.zip', 'cli.zip', 'sdk.tar.gz']
for entry, name in zip(manifest['sources'], names, strict=True):
    path = work / name
    if not path.exists():
        urllib.request.urlretrieve(entry['url'], path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != entry['sha256']:
        raise SystemExit(f'Digest mismatch: {name}')
    if name.endswith('.zip'):
        with zipfile.ZipFile(path) as archive:
            archive.extractall(work / name.removesuffix('.zip'))
    else:
        with tarfile.open(path) as archive:
            archive.extractall(work, filter='data')
cli = work / 'cli/wasi-vfs'
cli.chmod(0o755)
sdk = work / 'wasi-sdk-33.0-arm64-macos/bin'
objects = work / 'build'
command = [str(sdk / 'clang'), '-o', str(work / 'python-vfs.wasm'), '-z',
           'stack-size=16777216', '-Wl,--stack-first', '-Wl,--initial-memory=41943040',
           'Programs/python.o', 'libpython3.15.a']
command += [str(p.relative_to(objects)) for p in sorted((objects / 'Modules/_hacl').glob('*.a'))]
command += ['Modules/expat/libexpat.a', str(work / 'vfs/libwasi_vfs.a'), '-ldl',
            '-lwasi-emulated-signal', '-lwasi-emulated-getpid',
            '-lwasi-emulated-process-clocks', '-lpthread', '-lm']
subprocess.run(command, cwd=objects, check=True)
subprocess.run([str(cli), 'pack', str(work/'python-vfs.wasm'), '--mapdir',
                '/lib::' + str(work/'python/lib'), '-o', str(work/'python-packed.wasm')], check=True)
subprocess.run([str(sdk/'llvm-strip'), str(work/'python-packed.wasm'), '-o', str(work/'python.wasm')], check=True)
result = work/'python.wasm'
print(json.dumps({'path':str(result), 'sha256':hashlib.sha256(result.read_bytes()).hexdigest(),
                  'bytes':result.stat().st_size}, indent=2))
print('Verify the new artifact with the escape suite before replacing runtimes/python.wasm.')
