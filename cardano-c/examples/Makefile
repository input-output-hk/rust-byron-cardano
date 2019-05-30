CC=gcc
CFLAGS=-I.
LIB = ../../target/debug/libcardano_c.a

signed_transaction: signed_transaction.c $(LIB)
	$(CC) -o signed_transaction signed_transaction.c $(LIB) -lpthread -lm -ldl

.PHONY: clean

clean:
	rm signed_transaction
