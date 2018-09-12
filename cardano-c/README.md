# C binding for the cardano library

exports simple API to use in C library or to write bindings in
other languages.

# Cross compiling for different targets

To ease the process of cross-compiling to different plate-forms
and architectures we provide a build script,

```bash
./build.sh <TARGETS>
```

To see the list of supported platforms, see `rustup target list`.
Theoretically, all the targets are supported.

# Cross compiling for iOS

Use [cardano-lipo](https://github.com/TimNN/cargo-lipo)

# find linker for your targets

rust does not provide the linker for the targets, it is you to
provide it. For example, to cross compile for a given _`<target>`_:


- in `.cargo/config
  ```toml
  [target.<target>]
  linker = "/path/to/linker/for/<target>"
  ```
- then run build script:
  ```bash
  ./build.sh <target>
  ```

After successful completion of the script, you will find a directory `dist` at the root of this
crate with directories containing the differently built targets. In our example, something like:

```bash
$ tree dist/
dist/
└── cardano-c
    └── <target>
        ├── debug
        │   ├── libcardano_c.a
        │   └── libcardano_c.d
        └── release
            ├── libcardano_c.a
            └── libcardano_c.d
```

The `*.d` files are kept in case you want to integrate these in your build
and track changes to the rust library as well.
