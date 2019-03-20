#!/bin/sh

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

gcc -o test-cardano-c.$$ -I "${C_ROOT}" "${C_ROOT}test/test.c" "${C_ROOT}test/unity/unity.c" "${PROJECT_ROOT}target/debug/libcardano_c.a" -lpthread -lm -ldl
echo "######################################################################"
./test-cardano-c.$$
echo ""
echo "######################################################################"
rm test-cardano-c.$$


gcc -o test-cardano-c.$$ -I "${C_ROOT}" "${C_ROOT}test/test_bip39_entropy.c" "${C_ROOT}test/unity/unity.c" "${PROJECT_ROOT}target/debug/libcardano_c.a" -lpthread -lm -ldl
echo "######################################################################"
./test-cardano-c.$$
echo ""
echo "######################################################################"
rm test-cardano-c.$$
