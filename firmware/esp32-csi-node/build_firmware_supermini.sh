#!/usr/bin/env bash
# Build the ESP32-S3 CSI node firmware for the ESP32-S3 SuperMini board
# (4 MB flash, NO PSRAM).
#
# Usage:
#   ./build_firmware_supermini.sh                       # incremental build
#   ./build_firmware_supermini.sh clean                 # wipe build_supermini/
#   ./build_firmware_supermini.sh rebuild               # wipe + build
#   ./build_firmware_supermini.sh flash /dev/ttyACM0    # build + flash
#   ./build_firmware_supermini.sh monitor /dev/ttyACM0  # build + flash + monitor
#
# Env overrides:
#   IDF_PATH    — defaults to ~/esp/esp-idf
#   BUILD_DIR   — defaults to build_supermini

set -euo pipefail

IDF_PATH="${IDF_PATH:-$HOME/esp/esp-idf}"
BUILD_DIR="${BUILD_DIR:-build_supermini}"
SDKCONFIG_DEFAULTS="sdkconfig.defaults.4mb_supermini"
SDKCONFIG_OUT="${BUILD_DIR}/sdkconfig.supermini"

ACTION="${1:-build}"
PORT="${2:-}"

cd "$(dirname "$0")"

if [[ ! -f "${IDF_PATH}/export.sh" ]]; then
    echo "ERROR: ESP-IDF not found at ${IDF_PATH}" >&2
    exit 1
fi

if [[ ! -f "${SDKCONFIG_DEFAULTS}" ]]; then
    echo "ERROR: ${SDKCONFIG_DEFAULTS} missing — wrong working directory?" >&2
    exit 1
fi

if [[ "${ACTION}" == "clean" ]]; then
    echo "=== Wiping ${BUILD_DIR}/ ==="
    rm -rf "${BUILD_DIR}"
    echo "Done. To build: $0"
    exit 0
fi

# shellcheck disable=SC1091
. "${IDF_PATH}/export.sh" >/dev/null

if [[ "${ACTION}" == "rebuild" ]]; then
    echo "=== Wiping ${BUILD_DIR}/ ==="
    rm -rf "${BUILD_DIR}"
    ACTION="build"
fi

if [[ ! -f "${BUILD_DIR}/CMakeCache.txt" ]]; then
    echo "=== First-time configure: target esp32s3 | sdkconfig ${SDKCONFIG_DEFAULTS} ==="
    idf.py \
        -DSDKCONFIG_DEFAULTS="${SDKCONFIG_DEFAULTS}" \
        -DSDKCONFIG="${SDKCONFIG_OUT}" \
        -B "${BUILD_DIR}" \
        set-target esp32s3
else
    echo "=== Incremental build in ${BUILD_DIR}/ ==="
fi

idf.py -B "${BUILD_DIR}" build

BIN="${BUILD_DIR}/esp32-csi-node.bin"
if [[ -f "${BIN}" ]]; then
    size=$(stat -c %s "${BIN}")
    printf "=== Artifact: %s (%.0f KB) ===\n" "${BIN}" "$(echo "${size}/1024" | bc -l)"
fi

case "${ACTION}" in
    build)
        echo "Done. To flash: $0 flash /dev/ttyACM0"
        ;;
    flash)
        [[ -z "${PORT}" ]] && { echo "ERROR: flash requires a port (e.g. /dev/ttyACM0)" >&2; exit 2; }
        echo "=== Flashing ${PORT} ==="
        idf.py -B "${BUILD_DIR}" -p "${PORT}" flash
        ;;
    monitor)
        [[ -z "${PORT}" ]] && { echo "ERROR: monitor requires a port" >&2; exit 2; }
        echo "=== Flashing + monitoring ${PORT} ==="
        idf.py -B "${BUILD_DIR}" -p "${PORT}" flash monitor
        ;;
    *)
        echo "ERROR: unknown action '${ACTION}'. Use build | clean | rebuild | flash | monitor." >&2
        exit 2
        ;;
esac
