#!/usr/bin/env bash
set -u
usage() { echo "Usage: $0 [--simulate-missing <command>] [candidate]"; }
simulate=; candidate=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --help) usage; exit 0 ;;
    --simulate-missing) [ "$#" -ge 2 ] || { usage; exit 2; }; simulate=$2; shift 2 ;;
    moonbit|assemblyscript|cpp|go) candidate=$1; shift ;;
    *) usage; exit 2 ;;
  esac
done
command_path() {
  local command=$1 path
  if [ "$command" = moon ] && [ -x "$HOME/.moon/bin/moon" ]; then echo "$HOME/.moon/bin/moon"; return 0; fi
  if [ "$command" = clang++ ]; then
    if [ -n "${WASI_SDK_PATH:-}" ] && [ -x "$WASI_SDK_PATH/bin/clang++" ]; then echo "$WASI_SDK_PATH/bin/clang++"; return 0; fi
    [ -x 'C:/wasi-sdk/bin/clang++.exe' ] && { echo 'C:/wasi-sdk/bin/clang++.exe'; return 0; }
  fi
  if [ "$command" = wit-bindgen-go ]; then
    gopath=$(go env GOPATH 2>/dev/null || true)
    [ -n "$gopath" ] && [ -x "$gopath/bin/wit-bindgen-go" ] && { echo "$gopath/bin/wit-bindgen-go"; return 0; }
    [ -x "$gopath/bin/wit-bindgen-go.exe" ] && { echo "$gopath/bin/wit-bindgen-go.exe"; return 0; }
  fi
  path=$(command -v "$command" 2>/dev/null || true); [ -n "$path" ] && echo "$path"
}
install_verify() {
  case "$1" in
    moon) echo 'INSTALL: Set-ExecutionPolicy RemoteSigned -Scope CurrentUser; irm https://cli.moonbitlang.com/install/powershell.ps1 | iex'; echo 'VERIFY: moon version' ;;
    node|npx) echo 'INSTALL: MSI from https://nodejs.org/en/download'; echo 'VERIFY: node --version' ;;
    asc) echo 'INSTALL: npm install -g assemblyscript'; echo 'VERIFY: asc --version' ;;
    clang++) echo 'INSTALL: download wasi-sdk-33.0-x86_64-windows.tar.gz from https://github.com/WebAssembly/wasi-sdk/releases and extract it'; echo 'VERIFY: bin/clang++ --version' ;;
    go) echo 'INSTALL: MSI from https://go.dev/dl/'; echo 'VERIFY: go version' ;;
    wit-bindgen-go) echo 'INSTALL: go install go.bytecodealliance.org/cmd/wit-bindgen-go@latest'; echo 'VERIFY: wit-bindgen-go --version' ;;
    wasm-tools) echo 'INSTALL: cargo install --locked wasm-tools'; echo 'VERIFY: wasm-tools --version' ;;
    python3) echo 'INSTALL: installer from https://www.python.org/downloads/'; echo 'VERIFY: python --version' ;;
  esac
}
if [ -n "$simulate" ]; then
  case "$simulate" in
    moon|node|npx|asc|clang++|go|wit-bindgen-go|wasm-tools|python3)
      owner=shared
      case "$simulate" in moon) owner=moonbit ;; node|npx|asc) owner=assemblyscript ;; clang++) owner=cpp ;; go|wit-bindgen-go) owner=go ;; esac
      echo "BLOCKED: TOOLCHAIN $owner $simulate"; install_verify "$simulate"; exit 42 ;;
  esac
fi
check() {
  local command=$1 owner=$2
  if [ "$simulate" = "$command" ] || ! command_path "$command" >/dev/null; then
    echo "BLOCKED: TOOLCHAIN $owner $command"; install_verify "$command"; exit 42
  fi
}
for candidate in ${candidate:-moonbit assemblyscript cpp go}; do
  case "$candidate" in
    moonbit) check moon moonbit; check wasm-tools moonbit; check python3 moonbit ;;
    assemblyscript) check node assemblyscript; check npx assemblyscript; check asc assemblyscript; check wasm-tools assemblyscript; check python3 assemblyscript ;;
    cpp) check clang++ cpp; check wasm-tools cpp; check python3 cpp ;;
    go) check go go; check wit-bindgen-go go; check wasm-tools go; check python3 go ;;
  esac
done
echo 'PREREQUISITES OK'
