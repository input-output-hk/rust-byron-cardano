#! /bin/bash

set -e

# see `rustup target list` for the list of supported targets
TARGETS="${*}"

RUSTUP=$(which rustup)
CARGO="$(which cargo) --quiet"
MKDIR=$(which mkdir)
CP=$(which cp)

install_lib() {
    TARGET=${1}
    BUILD=${2}

    SOURCE_PATH=../target/${TARGET}/${BUILD}
    TARGET_PATH=./dist/cardano-c/${TARGET}/${BUILD}

    ${MKDIR} -p ${TARGET_PATH}

    echo " * installing to \`${TARGET_PATH}'"
    ${CP} ${SOURCE_PATH}/*.a ${TARGET_PATH}
    ${CP} ${SOURCE_PATH}/*.d ${TARGET_PATH}
    ${CP} ${SOURCE_PATH}/*.so ${TARGET_PATH}
}

rustup_target_add() {
    TARGET=${1}

    set +e
    ${RUSTUP} target list | grep ${TARGET} | grep --quiet "installed"
    set -e
    if [ ${?} -ne 0 ]; then
        echo " * installing toolchain's target: \`${TARGET}'"
        ${RUSTUP} target add ${TARGET}
    else
        echo " * toolchain's target \`${TARGET}' already installed"
    fi

}

for TARGET in ${TARGETS}; do
    echo "## compile library for target: \`${TARGET}'"
    echo ""

    rustup_target_add ${TARGET}
    echo " * compiling with debug symbols"
    ${CARGO} build           --target=${TARGET} --quiet
    echo " * compiling for release"
    ${CARGO} build --release --target=${TARGET} --quiet

    install_lib ${TARGET} "debug"
    install_lib ${TARGET} "release"
    echo ""
done
