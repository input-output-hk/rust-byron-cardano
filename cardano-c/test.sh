#!/bin/sh
# Run tests
# Set the VALGRIND variable to true to check for memory leaks
if [ -d target/debug ]; then
	PROJECT_ROOT="./"
elif [ -d ../target/debug ]; then
	PROJECT_ROOT="../"
else
	echo "no target debug directory"
	exit 1
fi

C_ROOT="${PROJECT_ROOT}cardano-c/"
C_LIB_A="${PROJECT_ROOT}target/debug/libcardano_c.a"

if [ ! -f "${C_LIB_A}" ]; then
	echo "no library file found. compile cardano-c first"
	exit 2
fi

if [ -t 1 ]; then
	UNITY_COLOR="-D UNITY_OUTPUT_COLOR"
fi

: "${VALGRIND:-false}"

for FILENAME in ${C_ROOT}test/*.c; do
    [ -e "$FILENAME" ] || continue
	gcc -o test-cardano-c.$$ -I "${C_ROOT}" "${FILENAME}" "${C_ROOT}test/unity/unity.c" "${PROJECT_ROOT}target/debug/libcardano_c.a" -lpthread -lm -ldl ${UNITY_COLOR} 
	echo "######################################################################"
	if [ "$VALGRIND" = true ] ; then
		valgrind ./test-cardano-c.$$
		else
		./test-cardano-c.$$
	fi
	echo ""
	echo "######################################################################"
	rm test-cardano-c.$$
done

