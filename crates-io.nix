{ lib, buildRustCrate, buildRustCrateHelpers }:
with buildRustCrateHelpers;
let inherit (lib.lists) fold;
    inherit (lib.attrsets) recursiveUpdate;
in
rec {

  crates.aho_corasick."0.6.8" = deps: { features?(features_.aho_corasick."0.6.8" deps {}) }: buildRustCrate {
    crateName = "aho-corasick";
    version = "0.6.8";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "04bz5m32ykyn946iwxgbrl8nwca7ssxsqma140hgmkchaay80nfr";
    libName = "aho_corasick";
    crateBin =
      [{  name = "aho-corasick-dot";  path = "src/main.rs"; }];
    dependencies = mapFeatures features ([
      (crates."memchr"."${deps."aho_corasick"."0.6.8"."memchr"}" deps)
    ]);
  };
  features_.aho_corasick."0.6.8" = deps: f: updateFeatures f (rec {
    aho_corasick."0.6.8".default = (f.aho_corasick."0.6.8".default or true);
    memchr."${deps.aho_corasick."0.6.8".memchr}".default = true;
  }) [
    (features_.memchr."${deps."aho_corasick"."0.6.8"."memchr"}" deps)
  ];


  crates.arrayvec."0.4.7" = deps: { features?(features_.arrayvec."0.4.7" deps {}) }: buildRustCrate {
    crateName = "arrayvec";
    version = "0.4.7";
    authors = [ "bluss" ];
    sha256 = "0fzgv7z1x1qnyd7j32vdcadk4k9wfx897y06mr3bw1yi52iqf4z4";
    dependencies = mapFeatures features ([
      (crates."nodrop"."${deps."arrayvec"."0.4.7"."nodrop"}" deps)
    ]);
    features = mkFeatures (features."arrayvec"."0.4.7" or {});
  };
  features_.arrayvec."0.4.7" = deps: f: updateFeatures f (rec {
    arrayvec = fold recursiveUpdate {} [
      { "0.4.7".default = (f.arrayvec."0.4.7".default or true); }
      { "0.4.7".serde =
        (f.arrayvec."0.4.7".serde or false) ||
        (f.arrayvec."0.4.7".serde-1 or false) ||
        (arrayvec."0.4.7"."serde-1" or false); }
      { "0.4.7".std =
        (f.arrayvec."0.4.7".std or false) ||
        (f.arrayvec."0.4.7".default or false) ||
        (arrayvec."0.4.7"."default" or false); }
    ];
    nodrop."${deps.arrayvec."0.4.7".nodrop}".default = (f.nodrop."${deps.arrayvec."0.4.7".nodrop}".default or false);
  }) [
    (features_.nodrop."${deps."arrayvec"."0.4.7"."nodrop"}" deps)
  ];


  crates.atty."0.2.11" = deps: { features?(features_.atty."0.2.11" deps {}) }: buildRustCrate {
    crateName = "atty";
    version = "0.2.11";
    authors = [ "softprops <d.tangren@gmail.com>" ];
    sha256 = "0by1bj2km9jxi4i4g76zzi76fc2rcm9934jpnyrqd95zw344pb20";
    dependencies = (if kernel == "redox" then mapFeatures features ([
      (crates."termion"."${deps."atty"."0.2.11"."termion"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."atty"."0.2.11"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."atty"."0.2.11"."winapi"}" deps)
    ]) else []);
  };
  features_.atty."0.2.11" = deps: f: updateFeatures f (rec {
    atty."0.2.11".default = (f.atty."0.2.11".default or true);
    libc."${deps.atty."0.2.11".libc}".default = (f.libc."${deps.atty."0.2.11".libc}".default or false);
    termion."${deps.atty."0.2.11".termion}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.atty."0.2.11".winapi}"."consoleapi" = true; }
      { "${deps.atty."0.2.11".winapi}"."minwinbase" = true; }
      { "${deps.atty."0.2.11".winapi}"."minwindef" = true; }
      { "${deps.atty."0.2.11".winapi}"."processenv" = true; }
      { "${deps.atty."0.2.11".winapi}"."winbase" = true; }
      { "${deps.atty."0.2.11".winapi}".default = true; }
    ];
  }) [
    (features_.termion."${deps."atty"."0.2.11"."termion"}" deps)
    (features_.libc."${deps."atty"."0.2.11"."libc"}" deps)
    (features_.winapi."${deps."atty"."0.2.11"."winapi"}" deps)
  ];


  crates.base64."0.9.3" = deps: { features?(features_.base64."0.9.3" deps {}) }: buildRustCrate {
    crateName = "base64";
    version = "0.9.3";
    authors = [ "Alice Maz <alice@alicemaz.com>" "Marshall Pierce <marshall@mpierce.org>" ];
    sha256 = "11hhz8ln4zbpn2h2gm9fbbb9j254wrd4fpmddlyah2rrnqsmmqkd";
    dependencies = mapFeatures features ([
      (crates."byteorder"."${deps."base64"."0.9.3"."byteorder"}" deps)
      (crates."safemem"."${deps."base64"."0.9.3"."safemem"}" deps)
    ]);
  };
  features_.base64."0.9.3" = deps: f: updateFeatures f (rec {
    base64."0.9.3".default = (f.base64."0.9.3".default or true);
    byteorder."${deps.base64."0.9.3".byteorder}".default = true;
    safemem."${deps.base64."0.9.3".safemem}".default = true;
  }) [
    (features_.byteorder."${deps."base64"."0.9.3"."byteorder"}" deps)
    (features_.safemem."${deps."base64"."0.9.3"."safemem"}" deps)
  ];


  crates.bitflags."1.0.4" = deps: { features?(features_.bitflags."1.0.4" deps {}) }: buildRustCrate {
    crateName = "bitflags";
    version = "1.0.4";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1g1wmz2001qmfrd37dnd5qiss5njrw26aywmg6yhkmkbyrhjxb08";
    features = mkFeatures (features."bitflags"."1.0.4" or {});
  };
  features_.bitflags."1.0.4" = deps: f: updateFeatures f (rec {
    bitflags."1.0.4".default = (f.bitflags."1.0.4".default or true);
  }) [];


  crates.byteorder."1.2.6" = deps: { features?(features_.byteorder."1.2.6" deps {}) }: buildRustCrate {
    crateName = "byteorder";
    version = "1.2.6";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "12p5ms2jsqr5l1d3fskpqzjvnn4b41pzwbjbz9zfaj22ndhkk87d";
    features = mkFeatures (features."byteorder"."1.2.6" or {});
  };
  features_.byteorder."1.2.6" = deps: f: updateFeatures f (rec {
    byteorder = fold recursiveUpdate {} [
      { "1.2.6".default = (f.byteorder."1.2.6".default or true); }
      { "1.2.6".std =
        (f.byteorder."1.2.6".std or false) ||
        (f.byteorder."1.2.6".default or false) ||
        (byteorder."1.2.6"."default" or false); }
    ];
  }) [];


  crates.bytes."0.4.10" = deps: { features?(features_.bytes."0.4.10" deps {}) }: buildRustCrate {
    crateName = "bytes";
    version = "0.4.10";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0g7c9qc6g5vjajha0ykxv63fmpg39i9w204j6pc1lknr99i7w19a";
    dependencies = mapFeatures features ([
      (crates."byteorder"."${deps."bytes"."0.4.10"."byteorder"}" deps)
      (crates."iovec"."${deps."bytes"."0.4.10"."iovec"}" deps)
    ]);
    features = mkFeatures (features."bytes"."0.4.10" or {});
  };
  features_.bytes."0.4.10" = deps: f: updateFeatures f (rec {
    byteorder = fold recursiveUpdate {} [
      { "${deps.bytes."0.4.10".byteorder}".default = true; }
      { "1.2.6".i128 =
        (f.byteorder."1.2.6".i128 or false) ||
        (bytes."0.4.10"."i128" or false) ||
        (f."bytes"."0.4.10"."i128" or false); }
    ];
    bytes."0.4.10".default = (f.bytes."0.4.10".default or true);
    iovec."${deps.bytes."0.4.10".iovec}".default = true;
  }) [
    (features_.byteorder."${deps."bytes"."0.4.10"."byteorder"}" deps)
    (features_.iovec."${deps."bytes"."0.4.10"."iovec"}" deps)
  ];


  crates.cbor_event."1.0.1" = deps: { features?(features_.cbor_event."1.0.1" deps {}) }: buildRustCrate {
    crateName = "cbor_event";
    version = "1.0.1";
    authors = [ "Nicolas Di Prima <nicolas@primetype.co.uk>" "Vincent Hanquez <vincent@typed.io>" ];
    sha256 = "17x8lxrkry2wgq4966ny9jp7q6dhra80nzq6j287rhrmqcx1dac9";
  };
  features_.cbor_event."1.0.1" = deps: f: updateFeatures f (rec {
    cbor_event."1.0.1".default = (f.cbor_event."1.0.1".default or true);
  }) [];


  crates.cfg_if."0.1.6" = deps: { features?(features_.cfg_if."0.1.6" deps {}) }: buildRustCrate {
    crateName = "cfg-if";
    version = "0.1.6";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "11qrix06wagkplyk908i3423ps9m9np6c4vbcq81s9fyl244xv3n";
  };
  features_.cfg_if."0.1.6" = deps: f: updateFeatures f (rec {
    cfg_if."0.1.6".default = (f.cfg_if."0.1.6".default or true);
  }) [];


  crates.cloudabi."0.0.3" = deps: { features?(features_.cloudabi."0.0.3" deps {}) }: buildRustCrate {
    crateName = "cloudabi";
    version = "0.0.3";
    authors = [ "Nuxi (https://nuxi.nl/) and contributors" ];
    sha256 = "1z9lby5sr6vslfd14d6igk03s7awf91mxpsfmsp3prxbxlk0x7h5";
    libPath = "cloudabi.rs";
    dependencies = mapFeatures features ([
    ]
      ++ (if features.cloudabi."0.0.3".bitflags or false then [ (crates.bitflags."1.0.4" deps) ] else []));
    features = mkFeatures (features."cloudabi"."0.0.3" or {});
  };
  features_.cloudabi."0.0.3" = deps: f: updateFeatures f (rec {
    bitflags."${deps.cloudabi."0.0.3".bitflags}".default = true;
    cloudabi = fold recursiveUpdate {} [
      { "0.0.3".bitflags =
        (f.cloudabi."0.0.3".bitflags or false) ||
        (f.cloudabi."0.0.3".default or false) ||
        (cloudabi."0.0.3"."default" or false); }
      { "0.0.3".default = (f.cloudabi."0.0.3".default or true); }
    ];
  }) [
    (features_.bitflags."${deps."cloudabi"."0.0.3"."bitflags"}" deps)
  ];


  crates.crossbeam_deque."0.6.1" = deps: { features?(features_.crossbeam_deque."0.6.1" deps {}) }: buildRustCrate {
    crateName = "crossbeam-deque";
    version = "0.6.1";
    authors = [ "The Crossbeam Project Developers" ];
    sha256 = "00n2179ci0w3aw1k579y3g13rhckl631m41c25q689li8i36416m";
    dependencies = mapFeatures features ([
      (crates."crossbeam_epoch"."${deps."crossbeam_deque"."0.6.1"."crossbeam_epoch"}" deps)
      (crates."crossbeam_utils"."${deps."crossbeam_deque"."0.6.1"."crossbeam_utils"}" deps)
    ]);
  };
  features_.crossbeam_deque."0.6.1" = deps: f: updateFeatures f (rec {
    crossbeam_deque."0.6.1".default = (f.crossbeam_deque."0.6.1".default or true);
    crossbeam_epoch."${deps.crossbeam_deque."0.6.1".crossbeam_epoch}".default = true;
    crossbeam_utils."${deps.crossbeam_deque."0.6.1".crossbeam_utils}".default = true;
  }) [
    (features_.crossbeam_epoch."${deps."crossbeam_deque"."0.6.1"."crossbeam_epoch"}" deps)
    (features_.crossbeam_utils."${deps."crossbeam_deque"."0.6.1"."crossbeam_utils"}" deps)
  ];


  crates.crossbeam_epoch."0.5.2" = deps: { features?(features_.crossbeam_epoch."0.5.2" deps {}) }: buildRustCrate {
    crateName = "crossbeam-epoch";
    version = "0.5.2";
    authors = [ "The Crossbeam Project Developers" ];
    sha256 = "1xv8ggicdjwsqsbawflyjwlq5nj4xks96yzp5w3sw9qby6l16wnd";
    dependencies = mapFeatures features ([
      (crates."arrayvec"."${deps."crossbeam_epoch"."0.5.2"."arrayvec"}" deps)
      (crates."cfg_if"."${deps."crossbeam_epoch"."0.5.2"."cfg_if"}" deps)
      (crates."crossbeam_utils"."${deps."crossbeam_epoch"."0.5.2"."crossbeam_utils"}" deps)
      (crates."memoffset"."${deps."crossbeam_epoch"."0.5.2"."memoffset"}" deps)
      (crates."scopeguard"."${deps."crossbeam_epoch"."0.5.2"."scopeguard"}" deps)
    ]
      ++ (if features.crossbeam_epoch."0.5.2".lazy_static or false then [ (crates.lazy_static."1.1.0" deps) ] else []));
    features = mkFeatures (features."crossbeam_epoch"."0.5.2" or {});
  };
  features_.crossbeam_epoch."0.5.2" = deps: f: updateFeatures f (rec {
    arrayvec = fold recursiveUpdate {} [
      { "${deps.crossbeam_epoch."0.5.2".arrayvec}".default = (f.arrayvec."${deps.crossbeam_epoch."0.5.2".arrayvec}".default or false); }
      { "0.4.7".use_union =
        (f.arrayvec."0.4.7".use_union or false) ||
        (crossbeam_epoch."0.5.2"."nightly" or false) ||
        (f."crossbeam_epoch"."0.5.2"."nightly" or false); }
    ];
    cfg_if."${deps.crossbeam_epoch."0.5.2".cfg_if}".default = true;
    crossbeam_epoch = fold recursiveUpdate {} [
      { "0.5.2".default = (f.crossbeam_epoch."0.5.2".default or true); }
      { "0.5.2".lazy_static =
        (f.crossbeam_epoch."0.5.2".lazy_static or false) ||
        (f.crossbeam_epoch."0.5.2".use_std or false) ||
        (crossbeam_epoch."0.5.2"."use_std" or false); }
      { "0.5.2".use_std =
        (f.crossbeam_epoch."0.5.2".use_std or false) ||
        (f.crossbeam_epoch."0.5.2".default or false) ||
        (crossbeam_epoch."0.5.2"."default" or false); }
    ];
    crossbeam_utils = fold recursiveUpdate {} [
      { "${deps.crossbeam_epoch."0.5.2".crossbeam_utils}".default = (f.crossbeam_utils."${deps.crossbeam_epoch."0.5.2".crossbeam_utils}".default or false); }
      { "0.5.0".use_std =
        (f.crossbeam_utils."0.5.0".use_std or false) ||
        (crossbeam_epoch."0.5.2"."use_std" or false) ||
        (f."crossbeam_epoch"."0.5.2"."use_std" or false); }
    ];
    lazy_static."${deps.crossbeam_epoch."0.5.2".lazy_static}".default = true;
    memoffset."${deps.crossbeam_epoch."0.5.2".memoffset}".default = true;
    scopeguard."${deps.crossbeam_epoch."0.5.2".scopeguard}".default = (f.scopeguard."${deps.crossbeam_epoch."0.5.2".scopeguard}".default or false);
  }) [
    (features_.arrayvec."${deps."crossbeam_epoch"."0.5.2"."arrayvec"}" deps)
    (features_.cfg_if."${deps."crossbeam_epoch"."0.5.2"."cfg_if"}" deps)
    (features_.crossbeam_utils."${deps."crossbeam_epoch"."0.5.2"."crossbeam_utils"}" deps)
    (features_.lazy_static."${deps."crossbeam_epoch"."0.5.2"."lazy_static"}" deps)
    (features_.memoffset."${deps."crossbeam_epoch"."0.5.2"."memoffset"}" deps)
    (features_.scopeguard."${deps."crossbeam_epoch"."0.5.2"."scopeguard"}" deps)
  ];


  crates.crossbeam_utils."0.5.0" = deps: { features?(features_.crossbeam_utils."0.5.0" deps {}) }: buildRustCrate {
    crateName = "crossbeam-utils";
    version = "0.5.0";
    authors = [ "The Crossbeam Project Developers" ];
    sha256 = "1sx0s9lnv9ja3q9l649w7rn23d7mgvb3cl08zx69vp9x4rdpxdpn";
    features = mkFeatures (features."crossbeam_utils"."0.5.0" or {});
  };
  features_.crossbeam_utils."0.5.0" = deps: f: updateFeatures f (rec {
    crossbeam_utils = fold recursiveUpdate {} [
      { "0.5.0".default = (f.crossbeam_utils."0.5.0".default or true); }
      { "0.5.0".use_std =
        (f.crossbeam_utils."0.5.0".use_std or false) ||
        (f.crossbeam_utils."0.5.0".default or false) ||
        (crossbeam_utils."0.5.0"."default" or false); }
    ];
  }) [];


  crates.cryptoxide."0.1.0" = deps: { features?(features_.cryptoxide."0.1.0" deps {}) }: buildRustCrate {
    crateName = "cryptoxide";
    version = "0.1.0";
    authors = [ "Vincent Hanquez <vincent@typed.io>" "Nicolas Di Prima <nicolas@prime-type.co.uk>" "The Rust-Crypto Project Developers" ];
    sha256 = "05ym36zpcywk9s2vj8az8d1w2x5harradnvrix5hiiav7ky8fq47";
  };
  features_.cryptoxide."0.1.0" = deps: f: updateFeatures f (rec {
    cryptoxide."0.1.0".default = (f.cryptoxide."0.1.0".default or true);
  }) [];


  crates.dtoa."0.4.3" = deps: { features?(features_.dtoa."0.4.3" deps {}) }: buildRustCrate {
    crateName = "dtoa";
    version = "0.4.3";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1xysdxdm24sk5ysim7lps4r2qaxfnj0sbakhmps4d42yssx30cw8";
  };
  features_.dtoa."0.4.3" = deps: f: updateFeatures f (rec {
    dtoa."0.4.3".default = (f.dtoa."0.4.3".default or true);
  }) [];


  crates.env_logger."0.5.13" = deps: { features?(features_.env_logger."0.5.13" deps {}) }: buildRustCrate {
    crateName = "env_logger";
    version = "0.5.13";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1q6vylngcz4bn088b4hvsl879l8yz1k2bma75waljb5p4h4kbb72";
    dependencies = mapFeatures features ([
      (crates."atty"."${deps."env_logger"."0.5.13"."atty"}" deps)
      (crates."humantime"."${deps."env_logger"."0.5.13"."humantime"}" deps)
      (crates."log"."${deps."env_logger"."0.5.13"."log"}" deps)
      (crates."termcolor"."${deps."env_logger"."0.5.13"."termcolor"}" deps)
    ]
      ++ (if features.env_logger."0.5.13".regex or false then [ (crates.regex."1.0.5" deps) ] else []));
    features = mkFeatures (features."env_logger"."0.5.13" or {});
  };
  features_.env_logger."0.5.13" = deps: f: updateFeatures f (rec {
    atty."${deps.env_logger."0.5.13".atty}".default = true;
    env_logger = fold recursiveUpdate {} [
      { "0.5.13".default = (f.env_logger."0.5.13".default or true); }
      { "0.5.13".regex =
        (f.env_logger."0.5.13".regex or false) ||
        (f.env_logger."0.5.13".default or false) ||
        (env_logger."0.5.13"."default" or false); }
    ];
    humantime."${deps.env_logger."0.5.13".humantime}".default = true;
    log = fold recursiveUpdate {} [
      { "${deps.env_logger."0.5.13".log}"."std" = true; }
      { "${deps.env_logger."0.5.13".log}".default = true; }
    ];
    regex."${deps.env_logger."0.5.13".regex}".default = true;
    termcolor."${deps.env_logger."0.5.13".termcolor}".default = true;
  }) [
    (features_.atty."${deps."env_logger"."0.5.13"."atty"}" deps)
    (features_.humantime."${deps."env_logger"."0.5.13"."humantime"}" deps)
    (features_.log."${deps."env_logger"."0.5.13"."log"}" deps)
    (features_.regex."${deps."env_logger"."0.5.13"."regex"}" deps)
    (features_.termcolor."${deps."env_logger"."0.5.13"."termcolor"}" deps)
  ];


  crates.fuchsia_zircon."0.3.3" = deps: { features?(features_.fuchsia_zircon."0.3.3" deps {}) }: buildRustCrate {
    crateName = "fuchsia-zircon";
    version = "0.3.3";
    authors = [ "Raph Levien <raph@google.com>" ];
    sha256 = "0jrf4shb1699r4la8z358vri8318w4mdi6qzfqy30p2ymjlca4gk";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."fuchsia_zircon"."0.3.3"."bitflags"}" deps)
      (crates."fuchsia_zircon_sys"."${deps."fuchsia_zircon"."0.3.3"."fuchsia_zircon_sys"}" deps)
    ]);
  };
  features_.fuchsia_zircon."0.3.3" = deps: f: updateFeatures f (rec {
    bitflags."${deps.fuchsia_zircon."0.3.3".bitflags}".default = true;
    fuchsia_zircon."0.3.3".default = (f.fuchsia_zircon."0.3.3".default or true);
    fuchsia_zircon_sys."${deps.fuchsia_zircon."0.3.3".fuchsia_zircon_sys}".default = true;
  }) [
    (features_.bitflags."${deps."fuchsia_zircon"."0.3.3"."bitflags"}" deps)
    (features_.fuchsia_zircon_sys."${deps."fuchsia_zircon"."0.3.3"."fuchsia_zircon_sys"}" deps)
  ];


  crates.fuchsia_zircon_sys."0.3.3" = deps: { features?(features_.fuchsia_zircon_sys."0.3.3" deps {}) }: buildRustCrate {
    crateName = "fuchsia-zircon-sys";
    version = "0.3.3";
    authors = [ "Raph Levien <raph@google.com>" ];
    sha256 = "08jp1zxrm9jbrr6l26bjal4dbm8bxfy57ickdgibsqxr1n9j3hf5";
  };
  features_.fuchsia_zircon_sys."0.3.3" = deps: f: updateFeatures f (rec {
    fuchsia_zircon_sys."0.3.3".default = (f.fuchsia_zircon_sys."0.3.3".default or true);
  }) [];


  crates.futures."0.1.25" = deps: { features?(features_.futures."0.1.25" deps {}) }: buildRustCrate {
    crateName = "futures";
    version = "0.1.25";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "1gdn9z3mi3jjzbxgvawqh90895130c3ydks55rshja0ncpn985q3";
    features = mkFeatures (features."futures"."0.1.25" or {});
  };
  features_.futures."0.1.25" = deps: f: updateFeatures f (rec {
    futures = fold recursiveUpdate {} [
      { "0.1.25".default = (f.futures."0.1.25".default or true); }
      { "0.1.25".use_std =
        (f.futures."0.1.25".use_std or false) ||
        (f.futures."0.1.25".default or false) ||
        (futures."0.1.25"."default" or false); }
      { "0.1.25".with-deprecated =
        (f.futures."0.1.25".with-deprecated or false) ||
        (f.futures."0.1.25".default or false) ||
        (futures."0.1.25"."default" or false); }
    ];
  }) [];


  crates.futures_cpupool."0.1.8" = deps: { features?(features_.futures_cpupool."0.1.8" deps {}) }: buildRustCrate {
    crateName = "futures-cpupool";
    version = "0.1.8";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "0ficd31n5ljiixy6x0vjglhq4fp0v1p4qzxm3v6ymsrb3z080l5c";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."futures_cpupool"."0.1.8"."futures"}" deps)
      (crates."num_cpus"."${deps."futures_cpupool"."0.1.8"."num_cpus"}" deps)
    ]);
    features = mkFeatures (features."futures_cpupool"."0.1.8" or {});
  };
  features_.futures_cpupool."0.1.8" = deps: f: updateFeatures f (rec {
    futures = fold recursiveUpdate {} [
      { "${deps.futures_cpupool."0.1.8".futures}"."use_std" = true; }
      { "${deps.futures_cpupool."0.1.8".futures}".default = (f.futures."${deps.futures_cpupool."0.1.8".futures}".default or false); }
      { "0.1.25".with-deprecated =
        (f.futures."0.1.25".with-deprecated or false) ||
        (futures_cpupool."0.1.8"."with-deprecated" or false) ||
        (f."futures_cpupool"."0.1.8"."with-deprecated" or false); }
    ];
    futures_cpupool = fold recursiveUpdate {} [
      { "0.1.8".default = (f.futures_cpupool."0.1.8".default or true); }
      { "0.1.8".with-deprecated =
        (f.futures_cpupool."0.1.8".with-deprecated or false) ||
        (f.futures_cpupool."0.1.8".default or false) ||
        (futures_cpupool."0.1.8"."default" or false); }
    ];
    num_cpus."${deps.futures_cpupool."0.1.8".num_cpus}".default = true;
  }) [
    (features_.futures."${deps."futures_cpupool"."0.1.8"."futures"}" deps)
    (features_.num_cpus."${deps."futures_cpupool"."0.1.8"."num_cpus"}" deps)
  ];


  crates.httparse."1.3.3" = deps: { features?(features_.httparse."1.3.3" deps {}) }: buildRustCrate {
    crateName = "httparse";
    version = "1.3.3";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "1jymxy4bl0mzgp2dx0pzqzbr72sw5jmr5sjqiry4xr88z4z9qlyx";
    build = "build.rs";
    features = mkFeatures (features."httparse"."1.3.3" or {});
  };
  features_.httparse."1.3.3" = deps: f: updateFeatures f (rec {
    httparse = fold recursiveUpdate {} [
      { "1.3.3".default = (f.httparse."1.3.3".default or true); }
      { "1.3.3".std =
        (f.httparse."1.3.3".std or false) ||
        (f.httparse."1.3.3".default or false) ||
        (httparse."1.3.3"."default" or false); }
    ];
  }) [];


  crates.humantime."1.1.1" = deps: { features?(features_.humantime."1.1.1" deps {}) }: buildRustCrate {
    crateName = "humantime";
    version = "1.1.1";
    authors = [ "Paul Colomiets <paul@colomiets.name>" ];
    sha256 = "1lzdfsfzdikcp1qb6wcdvnsdv16pmzr7p7cv171vnbnyz2lrwbgn";
    libPath = "src/lib.rs";
    dependencies = mapFeatures features ([
      (crates."quick_error"."${deps."humantime"."1.1.1"."quick_error"}" deps)
    ]);
  };
  features_.humantime."1.1.1" = deps: f: updateFeatures f (rec {
    humantime."1.1.1".default = (f.humantime."1.1.1".default or true);
    quick_error."${deps.humantime."1.1.1".quick_error}".default = true;
  }) [
    (features_.quick_error."${deps."humantime"."1.1.1"."quick_error"}" deps)
  ];


  crates.hyper."0.11.27" = deps: { features?(features_.hyper."0.11.27" deps {}) }: buildRustCrate {
    crateName = "hyper";
    version = "0.11.27";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "0q5as4lhvh31bzk4qm7j84snrmxyxyaqk040rfk72b42dn98mryi";
    dependencies = mapFeatures features ([
      (crates."base64"."${deps."hyper"."0.11.27"."base64"}" deps)
      (crates."bytes"."${deps."hyper"."0.11.27"."bytes"}" deps)
      (crates."futures"."${deps."hyper"."0.11.27"."futures"}" deps)
      (crates."futures_cpupool"."${deps."hyper"."0.11.27"."futures_cpupool"}" deps)
      (crates."httparse"."${deps."hyper"."0.11.27"."httparse"}" deps)
      (crates."iovec"."${deps."hyper"."0.11.27"."iovec"}" deps)
      (crates."language_tags"."${deps."hyper"."0.11.27"."language_tags"}" deps)
      (crates."log"."${deps."hyper"."0.11.27"."log"}" deps)
      (crates."mime"."${deps."hyper"."0.11.27"."mime"}" deps)
      (crates."net2"."${deps."hyper"."0.11.27"."net2"}" deps)
      (crates."percent_encoding"."${deps."hyper"."0.11.27"."percent_encoding"}" deps)
      (crates."relay"."${deps."hyper"."0.11.27"."relay"}" deps)
      (crates."time"."${deps."hyper"."0.11.27"."time"}" deps)
      (crates."tokio_core"."${deps."hyper"."0.11.27"."tokio_core"}" deps)
      (crates."tokio_io"."${deps."hyper"."0.11.27"."tokio_io"}" deps)
      (crates."tokio_service"."${deps."hyper"."0.11.27"."tokio_service"}" deps)
      (crates."unicase"."${deps."hyper"."0.11.27"."unicase"}" deps)
      (crates."want"."${deps."hyper"."0.11.27"."want"}" deps)
    ]
      ++ (if features.hyper."0.11.27".tokio-proto or false then [ (crates.tokio_proto."0.1.1" deps) ] else []));
    features = mkFeatures (features."hyper"."0.11.27" or {});
  };
  features_.hyper."0.11.27" = deps: f: updateFeatures f (rec {
    base64."${deps.hyper."0.11.27".base64}".default = true;
    bytes."${deps.hyper."0.11.27".bytes}".default = true;
    futures."${deps.hyper."0.11.27".futures}".default = true;
    futures_cpupool."${deps.hyper."0.11.27".futures_cpupool}".default = true;
    httparse."${deps.hyper."0.11.27".httparse}".default = true;
    hyper = fold recursiveUpdate {} [
      { "0.11.27".default = (f.hyper."0.11.27".default or true); }
      { "0.11.27".http =
        (f.hyper."0.11.27".http or false) ||
        (f.hyper."0.11.27".compat or false) ||
        (hyper."0.11.27"."compat" or false); }
      { "0.11.27".server-proto =
        (f.hyper."0.11.27".server-proto or false) ||
        (f.hyper."0.11.27".default or false) ||
        (hyper."0.11.27"."default" or false); }
      { "0.11.27".tokio-proto =
        (f.hyper."0.11.27".tokio-proto or false) ||
        (f.hyper."0.11.27".server-proto or false) ||
        (hyper."0.11.27"."server-proto" or false); }
    ];
    iovec."${deps.hyper."0.11.27".iovec}".default = true;
    language_tags."${deps.hyper."0.11.27".language_tags}".default = true;
    log."${deps.hyper."0.11.27".log}".default = true;
    mime."${deps.hyper."0.11.27".mime}".default = true;
    net2."${deps.hyper."0.11.27".net2}".default = true;
    percent_encoding."${deps.hyper."0.11.27".percent_encoding}".default = true;
    relay."${deps.hyper."0.11.27".relay}".default = true;
    time."${deps.hyper."0.11.27".time}".default = true;
    tokio_core."${deps.hyper."0.11.27".tokio_core}".default = true;
    tokio_io."${deps.hyper."0.11.27".tokio_io}".default = true;
    tokio_proto."${deps.hyper."0.11.27".tokio_proto}".default = true;
    tokio_service."${deps.hyper."0.11.27".tokio_service}".default = true;
    unicase."${deps.hyper."0.11.27".unicase}".default = true;
    want."${deps.hyper."0.11.27".want}".default = true;
  }) [
    (features_.base64."${deps."hyper"."0.11.27"."base64"}" deps)
    (features_.bytes."${deps."hyper"."0.11.27"."bytes"}" deps)
    (features_.futures."${deps."hyper"."0.11.27"."futures"}" deps)
    (features_.futures_cpupool."${deps."hyper"."0.11.27"."futures_cpupool"}" deps)
    (features_.httparse."${deps."hyper"."0.11.27"."httparse"}" deps)
    (features_.iovec."${deps."hyper"."0.11.27"."iovec"}" deps)
    (features_.language_tags."${deps."hyper"."0.11.27"."language_tags"}" deps)
    (features_.log."${deps."hyper"."0.11.27"."log"}" deps)
    (features_.mime."${deps."hyper"."0.11.27"."mime"}" deps)
    (features_.net2."${deps."hyper"."0.11.27"."net2"}" deps)
    (features_.percent_encoding."${deps."hyper"."0.11.27"."percent_encoding"}" deps)
    (features_.relay."${deps."hyper"."0.11.27"."relay"}" deps)
    (features_.time."${deps."hyper"."0.11.27"."time"}" deps)
    (features_.tokio_core."${deps."hyper"."0.11.27"."tokio_core"}" deps)
    (features_.tokio_io."${deps."hyper"."0.11.27"."tokio_io"}" deps)
    (features_.tokio_proto."${deps."hyper"."0.11.27"."tokio_proto"}" deps)
    (features_.tokio_service."${deps."hyper"."0.11.27"."tokio_service"}" deps)
    (features_.unicase."${deps."hyper"."0.11.27"."unicase"}" deps)
    (features_.want."${deps."hyper"."0.11.27"."want"}" deps)
  ];


  crates.iovec."0.1.2" = deps: { features?(features_.iovec."0.1.2" deps {}) }: buildRustCrate {
    crateName = "iovec";
    version = "0.1.2";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0vjymmb7wj4v4kza5jjn48fcdb85j3k37y7msjl3ifz0p9yiyp2r";
    dependencies = (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."iovec"."0.1.2"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."iovec"."0.1.2"."winapi"}" deps)
    ]) else []);
  };
  features_.iovec."0.1.2" = deps: f: updateFeatures f (rec {
    iovec."0.1.2".default = (f.iovec."0.1.2".default or true);
    libc."${deps.iovec."0.1.2".libc}".default = true;
    winapi."${deps.iovec."0.1.2".winapi}".default = true;
  }) [
    (features_.libc."${deps."iovec"."0.1.2"."libc"}" deps)
    (features_.winapi."${deps."iovec"."0.1.2"."winapi"}" deps)
  ];


  crates.itoa."0.4.3" = deps: { features?(features_.itoa."0.4.3" deps {}) }: buildRustCrate {
    crateName = "itoa";
    version = "0.4.3";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "0zadimmdgvili3gdwxqg7ljv3r4wcdg1kkdfp9nl15vnm23vrhy1";
    features = mkFeatures (features."itoa"."0.4.3" or {});
  };
  features_.itoa."0.4.3" = deps: f: updateFeatures f (rec {
    itoa = fold recursiveUpdate {} [
      { "0.4.3".default = (f.itoa."0.4.3".default or true); }
      { "0.4.3".std =
        (f.itoa."0.4.3".std or false) ||
        (f.itoa."0.4.3".default or false) ||
        (itoa."0.4.3"."default" or false); }
    ];
  }) [];


  crates.kernel32_sys."0.2.2" = deps: { features?(features_.kernel32_sys."0.2.2" deps {}) }: buildRustCrate {
    crateName = "kernel32-sys";
    version = "0.2.2";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1lrw1hbinyvr6cp28g60z97w32w8vsk6pahk64pmrv2fmby8srfj";
    libName = "kernel32";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."winapi"."${deps."kernel32_sys"."0.2.2"."winapi"}" deps)
    ]);

    buildDependencies = mapFeatures features ([
      (crates."winapi_build"."${deps."kernel32_sys"."0.2.2"."winapi_build"}" deps)
    ]);
  };
  features_.kernel32_sys."0.2.2" = deps: f: updateFeatures f (rec {
    kernel32_sys."0.2.2".default = (f.kernel32_sys."0.2.2".default or true);
    winapi."${deps.kernel32_sys."0.2.2".winapi}".default = true;
    winapi_build."${deps.kernel32_sys."0.2.2".winapi_build}".default = true;
  }) [
    (features_.winapi."${deps."kernel32_sys"."0.2.2"."winapi"}" deps)
    (features_.winapi_build."${deps."kernel32_sys"."0.2.2"."winapi_build"}" deps)
  ];


  crates.language_tags."0.2.2" = deps: { features?(features_.language_tags."0.2.2" deps {}) }: buildRustCrate {
    crateName = "language-tags";
    version = "0.2.2";
    authors = [ "Pyfisch <pyfisch@gmail.com>" ];
    sha256 = "1zkrdzsqzzc7509kd7nngdwrp461glm2g09kqpzaqksp82frjdvy";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."language_tags"."0.2.2" or {});
  };
  features_.language_tags."0.2.2" = deps: f: updateFeatures f (rec {
    language_tags = fold recursiveUpdate {} [
      { "0.2.2".default = (f.language_tags."0.2.2".default or true); }
      { "0.2.2".heapsize =
        (f.language_tags."0.2.2".heapsize or false) ||
        (f.language_tags."0.2.2".heap_size or false) ||
        (language_tags."0.2.2"."heap_size" or false); }
      { "0.2.2".heapsize_plugin =
        (f.language_tags."0.2.2".heapsize_plugin or false) ||
        (f.language_tags."0.2.2".heap_size or false) ||
        (language_tags."0.2.2"."heap_size" or false); }
    ];
  }) [];


  crates.lazy_static."1.1.0" = deps: { features?(features_.lazy_static."1.1.0" deps {}) }: buildRustCrate {
    crateName = "lazy_static";
    version = "1.1.0";
    authors = [ "Marvin Löbel <loebel.marvin@gmail.com>" ];
    sha256 = "1da2b6nxfc2l547qgl9kd1pn9sh1af96a6qx6xw8xdnv6hh5fag0";
    build = "build.rs";
    dependencies = mapFeatures features ([
]);

    buildDependencies = mapFeatures features ([
      (crates."version_check"."${deps."lazy_static"."1.1.0"."version_check"}" deps)
    ]);
    features = mkFeatures (features."lazy_static"."1.1.0" or {});
  };
  features_.lazy_static."1.1.0" = deps: f: updateFeatures f (rec {
    lazy_static = fold recursiveUpdate {} [
      { "1.1.0".default = (f.lazy_static."1.1.0".default or true); }
      { "1.1.0".nightly =
        (f.lazy_static."1.1.0".nightly or false) ||
        (f.lazy_static."1.1.0".spin_no_std or false) ||
        (lazy_static."1.1.0"."spin_no_std" or false); }
      { "1.1.0".spin =
        (f.lazy_static."1.1.0".spin or false) ||
        (f.lazy_static."1.1.0".spin_no_std or false) ||
        (lazy_static."1.1.0"."spin_no_std" or false); }
    ];
    version_check."${deps.lazy_static."1.1.0".version_check}".default = true;
  }) [
    (features_.version_check."${deps."lazy_static"."1.1.0"."version_check"}" deps)
  ];


  crates.lazycell."1.2.0" = deps: { features?(features_.lazycell."1.2.0" deps {}) }: buildRustCrate {
    crateName = "lazycell";
    version = "1.2.0";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" "Nikita Pekin <contact@nikitapek.in>" ];
    sha256 = "1lzdb3q17yjihw9hksynxgyg8wbph1h791wff8rrf1c2aqjwhmax";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."lazycell"."1.2.0" or {});
  };
  features_.lazycell."1.2.0" = deps: f: updateFeatures f (rec {
    lazycell = fold recursiveUpdate {} [
      { "1.2.0".clippy =
        (f.lazycell."1.2.0".clippy or false) ||
        (f.lazycell."1.2.0".nightly-testing or false) ||
        (lazycell."1.2.0"."nightly-testing" or false); }
      { "1.2.0".default = (f.lazycell."1.2.0".default or true); }
      { "1.2.0".nightly =
        (f.lazycell."1.2.0".nightly or false) ||
        (f.lazycell."1.2.0".nightly-testing or false) ||
        (lazycell."1.2.0"."nightly-testing" or false); }
    ];
  }) [];


  crates.libc."0.2.43" = deps: { features?(features_.libc."0.2.43" deps {}) }: buildRustCrate {
    crateName = "libc";
    version = "0.2.43";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0pshydmsq71kl9276zc2928ld50sp524ixcqkcqsgq410dx6c50b";
    features = mkFeatures (features."libc"."0.2.43" or {});
  };
  features_.libc."0.2.43" = deps: f: updateFeatures f (rec {
    libc = fold recursiveUpdate {} [
      { "0.2.43".default = (f.libc."0.2.43".default or true); }
      { "0.2.43".use_std =
        (f.libc."0.2.43".use_std or false) ||
        (f.libc."0.2.43".default or false) ||
        (libc."0.2.43"."default" or false); }
    ];
  }) [];


  crates.linked_hash_map."0.5.1" = deps: { features?(features_.linked_hash_map."0.5.1" deps {}) }: buildRustCrate {
    crateName = "linked-hash-map";
    version = "0.5.1";
    authors = [ "Stepan Koltsov <stepan.koltsov@gmail.com>" "Andrew Paseltiner <apaseltiner@gmail.com>" ];
    sha256 = "1f29c7j53z7w5v0g115yii9dmmbsahr93ak375g48vi75v3p4030";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."linked_hash_map"."0.5.1" or {});
  };
  features_.linked_hash_map."0.5.1" = deps: f: updateFeatures f (rec {
    linked_hash_map = fold recursiveUpdate {} [
      { "0.5.1".default = (f.linked_hash_map."0.5.1".default or true); }
      { "0.5.1".heapsize =
        (f.linked_hash_map."0.5.1".heapsize or false) ||
        (f.linked_hash_map."0.5.1".heapsize_impl or false) ||
        (linked_hash_map."0.5.1"."heapsize_impl" or false); }
      { "0.5.1".serde =
        (f.linked_hash_map."0.5.1".serde or false) ||
        (f.linked_hash_map."0.5.1".serde_impl or false) ||
        (linked_hash_map."0.5.1"."serde_impl" or false); }
      { "0.5.1".serde_test =
        (f.linked_hash_map."0.5.1".serde_test or false) ||
        (f.linked_hash_map."0.5.1".serde_impl or false) ||
        (linked_hash_map."0.5.1"."serde_impl" or false); }
    ];
  }) [];


  crates.lock_api."0.1.4" = deps: { features?(features_.lock_api."0.1.4" deps {}) }: buildRustCrate {
    crateName = "lock_api";
    version = "0.1.4";
    authors = [ "Amanieu d'Antras <amanieu@gmail.com>" ];
    sha256 = "055dl3crjiid0bsmrwp3z3s6ypgscv4zsqgdj0pmhxr6zaas1da2";
    dependencies = mapFeatures features ([
      (crates."scopeguard"."${deps."lock_api"."0.1.4"."scopeguard"}" deps)
    ]
      ++ (if features.lock_api."0.1.4".owning_ref or false then [ (crates.owning_ref."0.3.3" deps) ] else []));
    features = mkFeatures (features."lock_api"."0.1.4" or {});
  };
  features_.lock_api."0.1.4" = deps: f: updateFeatures f (rec {
    lock_api."0.1.4".default = (f.lock_api."0.1.4".default or true);
    owning_ref."${deps.lock_api."0.1.4".owning_ref}".default = true;
    scopeguard."${deps.lock_api."0.1.4".scopeguard}".default = (f.scopeguard."${deps.lock_api."0.1.4".scopeguard}".default or false);
  }) [
    (features_.owning_ref."${deps."lock_api"."0.1.4"."owning_ref"}" deps)
    (features_.scopeguard."${deps."lock_api"."0.1.4"."scopeguard"}" deps)
  ];


  crates.log."0.3.9" = deps: { features?(features_.log."0.3.9" deps {}) }: buildRustCrate {
    crateName = "log";
    version = "0.3.9";
    authors = [ "The Rust Project Developers" ];
    sha256 = "19i9pwp7lhaqgzangcpw00kc3zsgcqcx84crv07xgz3v7d3kvfa2";
    dependencies = mapFeatures features ([
      (crates."log"."${deps."log"."0.3.9"."log"}" deps)
    ]);
    features = mkFeatures (features."log"."0.3.9" or {});
  };
  features_.log."0.3.9" = deps: f: updateFeatures f (rec {
    log = fold recursiveUpdate {} [
      { "${deps.log."0.3.9".log}".default = true; }
      { "0.3.9".default = (f.log."0.3.9".default or true); }
      { "0.3.9".use_std =
        (f.log."0.3.9".use_std or false) ||
        (f.log."0.3.9".default or false) ||
        (log."0.3.9"."default" or false); }
      { "0.4.5".max_level_debug =
        (f.log."0.4.5".max_level_debug or false) ||
        (log."0.3.9"."max_level_debug" or false) ||
        (f."log"."0.3.9"."max_level_debug" or false); }
      { "0.4.5".max_level_error =
        (f.log."0.4.5".max_level_error or false) ||
        (log."0.3.9"."max_level_error" or false) ||
        (f."log"."0.3.9"."max_level_error" or false); }
      { "0.4.5".max_level_info =
        (f.log."0.4.5".max_level_info or false) ||
        (log."0.3.9"."max_level_info" or false) ||
        (f."log"."0.3.9"."max_level_info" or false); }
      { "0.4.5".max_level_off =
        (f.log."0.4.5".max_level_off or false) ||
        (log."0.3.9"."max_level_off" or false) ||
        (f."log"."0.3.9"."max_level_off" or false); }
      { "0.4.5".max_level_trace =
        (f.log."0.4.5".max_level_trace or false) ||
        (log."0.3.9"."max_level_trace" or false) ||
        (f."log"."0.3.9"."max_level_trace" or false); }
      { "0.4.5".max_level_warn =
        (f.log."0.4.5".max_level_warn or false) ||
        (log."0.3.9"."max_level_warn" or false) ||
        (f."log"."0.3.9"."max_level_warn" or false); }
      { "0.4.5".release_max_level_debug =
        (f.log."0.4.5".release_max_level_debug or false) ||
        (log."0.3.9"."release_max_level_debug" or false) ||
        (f."log"."0.3.9"."release_max_level_debug" or false); }
      { "0.4.5".release_max_level_error =
        (f.log."0.4.5".release_max_level_error or false) ||
        (log."0.3.9"."release_max_level_error" or false) ||
        (f."log"."0.3.9"."release_max_level_error" or false); }
      { "0.4.5".release_max_level_info =
        (f.log."0.4.5".release_max_level_info or false) ||
        (log."0.3.9"."release_max_level_info" or false) ||
        (f."log"."0.3.9"."release_max_level_info" or false); }
      { "0.4.5".release_max_level_off =
        (f.log."0.4.5".release_max_level_off or false) ||
        (log."0.3.9"."release_max_level_off" or false) ||
        (f."log"."0.3.9"."release_max_level_off" or false); }
      { "0.4.5".release_max_level_trace =
        (f.log."0.4.5".release_max_level_trace or false) ||
        (log."0.3.9"."release_max_level_trace" or false) ||
        (f."log"."0.3.9"."release_max_level_trace" or false); }
      { "0.4.5".release_max_level_warn =
        (f.log."0.4.5".release_max_level_warn or false) ||
        (log."0.3.9"."release_max_level_warn" or false) ||
        (f."log"."0.3.9"."release_max_level_warn" or false); }
      { "0.4.5".std =
        (f.log."0.4.5".std or false) ||
        (log."0.3.9"."use_std" or false) ||
        (f."log"."0.3.9"."use_std" or false); }
    ];
  }) [
    (features_.log."${deps."log"."0.3.9"."log"}" deps)
  ];


  crates.log."0.4.5" = deps: { features?(features_.log."0.4.5" deps {}) }: buildRustCrate {
    crateName = "log";
    version = "0.4.5";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1hdcj17al94ga90q7jx2y1rmxi68n3akra1awv3hr3s9b9zipgq6";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."log"."0.4.5"."cfg_if"}" deps)
    ]);
    features = mkFeatures (features."log"."0.4.5" or {});
  };
  features_.log."0.4.5" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.log."0.4.5".cfg_if}".default = true;
    log."0.4.5".default = (f.log."0.4.5".default or true);
  }) [
    (features_.cfg_if."${deps."log"."0.4.5"."cfg_if"}" deps)
  ];


  crates.memchr."2.1.0" = deps: { features?(features_.memchr."2.1.0" deps {}) }: buildRustCrate {
    crateName = "memchr";
    version = "2.1.0";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" "bluss" ];
    sha256 = "02w1fc5z1ccx8fbzgcr0mpk0xf2i9g4vbx9q5c2g8pjddbaqvjjq";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."memchr"."2.1.0"."cfg_if"}" deps)
    ]
      ++ (if features.memchr."2.1.0".libc or false then [ (crates.libc."0.2.43" deps) ] else []));

    buildDependencies = mapFeatures features ([
      (crates."version_check"."${deps."memchr"."2.1.0"."version_check"}" deps)
    ]);
    features = mkFeatures (features."memchr"."2.1.0" or {});
  };
  features_.memchr."2.1.0" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.memchr."2.1.0".cfg_if}".default = true;
    libc = fold recursiveUpdate {} [
      { "${deps.memchr."2.1.0".libc}".default = (f.libc."${deps.memchr."2.1.0".libc}".default or false); }
      { "0.2.43".use_std =
        (f.libc."0.2.43".use_std or false) ||
        (memchr."2.1.0"."use_std" or false) ||
        (f."memchr"."2.1.0"."use_std" or false); }
    ];
    memchr = fold recursiveUpdate {} [
      { "2.1.0".default = (f.memchr."2.1.0".default or true); }
      { "2.1.0".libc =
        (f.memchr."2.1.0".libc or false) ||
        (f.memchr."2.1.0".default or false) ||
        (memchr."2.1.0"."default" or false) ||
        (f.memchr."2.1.0".use_std or false) ||
        (memchr."2.1.0"."use_std" or false); }
      { "2.1.0".use_std =
        (f.memchr."2.1.0".use_std or false) ||
        (f.memchr."2.1.0".default or false) ||
        (memchr."2.1.0"."default" or false); }
    ];
    version_check."${deps.memchr."2.1.0".version_check}".default = true;
  }) [
    (features_.cfg_if."${deps."memchr"."2.1.0"."cfg_if"}" deps)
    (features_.libc."${deps."memchr"."2.1.0"."libc"}" deps)
    (features_.version_check."${deps."memchr"."2.1.0"."version_check"}" deps)
  ];


  crates.memoffset."0.2.1" = deps: { features?(features_.memoffset."0.2.1" deps {}) }: buildRustCrate {
    crateName = "memoffset";
    version = "0.2.1";
    authors = [ "Gilad Naaman <gilad.naaman@gmail.com>" ];
    sha256 = "00vym01jk9slibq2nsiilgffp7n6k52a4q3n4dqp0xf5kzxvffcf";
  };
  features_.memoffset."0.2.1" = deps: f: updateFeatures f (rec {
    memoffset."0.2.1".default = (f.memoffset."0.2.1".default or true);
  }) [];


  crates.mime."0.3.12" = deps: { features?(features_.mime."0.3.12" deps {}) }: buildRustCrate {
    crateName = "mime";
    version = "0.3.12";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "0lmcwkmxwbla9457w9ak13cfgqxfyn5wa1syjy1kll2ras5xifvh";
    dependencies = mapFeatures features ([
      (crates."unicase"."${deps."mime"."0.3.12"."unicase"}" deps)
    ]);
  };
  features_.mime."0.3.12" = deps: f: updateFeatures f (rec {
    mime."0.3.12".default = (f.mime."0.3.12".default or true);
    unicase."${deps.mime."0.3.12".unicase}".default = true;
  }) [
    (features_.unicase."${deps."mime"."0.3.12"."unicase"}" deps)
  ];


  crates.mio."0.6.16" = deps: { features?(features_.mio."0.6.16" deps {}) }: buildRustCrate {
    crateName = "mio";
    version = "0.6.16";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "14vyrlmf0w984pi7ad9qvmlfj6vrb0wn6i8ik9j87w5za2r3rban";
    dependencies = mapFeatures features ([
      (crates."iovec"."${deps."mio"."0.6.16"."iovec"}" deps)
      (crates."lazycell"."${deps."mio"."0.6.16"."lazycell"}" deps)
      (crates."log"."${deps."mio"."0.6.16"."log"}" deps)
      (crates."net2"."${deps."mio"."0.6.16"."net2"}" deps)
      (crates."slab"."${deps."mio"."0.6.16"."slab"}" deps)
    ])
      ++ (if kernel == "fuchsia" then mapFeatures features ([
      (crates."fuchsia_zircon"."${deps."mio"."0.6.16"."fuchsia_zircon"}" deps)
      (crates."fuchsia_zircon_sys"."${deps."mio"."0.6.16"."fuchsia_zircon_sys"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."mio"."0.6.16"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."kernel32_sys"."${deps."mio"."0.6.16"."kernel32_sys"}" deps)
      (crates."miow"."${deps."mio"."0.6.16"."miow"}" deps)
      (crates."winapi"."${deps."mio"."0.6.16"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."mio"."0.6.16" or {});
  };
  features_.mio."0.6.16" = deps: f: updateFeatures f (rec {
    fuchsia_zircon."${deps.mio."0.6.16".fuchsia_zircon}".default = true;
    fuchsia_zircon_sys."${deps.mio."0.6.16".fuchsia_zircon_sys}".default = true;
    iovec."${deps.mio."0.6.16".iovec}".default = true;
    kernel32_sys."${deps.mio."0.6.16".kernel32_sys}".default = true;
    lazycell."${deps.mio."0.6.16".lazycell}".default = true;
    libc."${deps.mio."0.6.16".libc}".default = true;
    log."${deps.mio."0.6.16".log}".default = true;
    mio = fold recursiveUpdate {} [
      { "0.6.16".default = (f.mio."0.6.16".default or true); }
      { "0.6.16".with-deprecated =
        (f.mio."0.6.16".with-deprecated or false) ||
        (f.mio."0.6.16".default or false) ||
        (mio."0.6.16"."default" or false); }
    ];
    miow."${deps.mio."0.6.16".miow}".default = true;
    net2."${deps.mio."0.6.16".net2}".default = true;
    slab."${deps.mio."0.6.16".slab}".default = true;
    winapi."${deps.mio."0.6.16".winapi}".default = true;
  }) [
    (features_.iovec."${deps."mio"."0.6.16"."iovec"}" deps)
    (features_.lazycell."${deps."mio"."0.6.16"."lazycell"}" deps)
    (features_.log."${deps."mio"."0.6.16"."log"}" deps)
    (features_.net2."${deps."mio"."0.6.16"."net2"}" deps)
    (features_.slab."${deps."mio"."0.6.16"."slab"}" deps)
    (features_.fuchsia_zircon."${deps."mio"."0.6.16"."fuchsia_zircon"}" deps)
    (features_.fuchsia_zircon_sys."${deps."mio"."0.6.16"."fuchsia_zircon_sys"}" deps)
    (features_.libc."${deps."mio"."0.6.16"."libc"}" deps)
    (features_.kernel32_sys."${deps."mio"."0.6.16"."kernel32_sys"}" deps)
    (features_.miow."${deps."mio"."0.6.16"."miow"}" deps)
    (features_.winapi."${deps."mio"."0.6.16"."winapi"}" deps)
  ];


  crates.mio_uds."0.6.7" = deps: { features?(features_.mio_uds."0.6.7" deps {}) }: buildRustCrate {
    crateName = "mio-uds";
    version = "0.6.7";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "1gff9908pvvysv7zgxvyxy7x34fnhs088cr0j8mgwj8j24mswrhm";
    dependencies = (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."iovec"."${deps."mio_uds"."0.6.7"."iovec"}" deps)
      (crates."libc"."${deps."mio_uds"."0.6.7"."libc"}" deps)
      (crates."mio"."${deps."mio_uds"."0.6.7"."mio"}" deps)
    ]) else []);
  };
  features_.mio_uds."0.6.7" = deps: f: updateFeatures f (rec {
    iovec."${deps.mio_uds."0.6.7".iovec}".default = true;
    libc."${deps.mio_uds."0.6.7".libc}".default = true;
    mio."${deps.mio_uds."0.6.7".mio}".default = true;
    mio_uds."0.6.7".default = (f.mio_uds."0.6.7".default or true);
  }) [
    (features_.iovec."${deps."mio_uds"."0.6.7"."iovec"}" deps)
    (features_.libc."${deps."mio_uds"."0.6.7"."libc"}" deps)
    (features_.mio."${deps."mio_uds"."0.6.7"."mio"}" deps)
  ];


  crates.miow."0.2.1" = deps: { features?(features_.miow."0.2.1" deps {}) }: buildRustCrate {
    crateName = "miow";
    version = "0.2.1";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "14f8zkc6ix7mkyis1vsqnim8m29b6l55abkba3p2yz7j1ibcvrl0";
    dependencies = mapFeatures features ([
      (crates."kernel32_sys"."${deps."miow"."0.2.1"."kernel32_sys"}" deps)
      (crates."net2"."${deps."miow"."0.2.1"."net2"}" deps)
      (crates."winapi"."${deps."miow"."0.2.1"."winapi"}" deps)
      (crates."ws2_32_sys"."${deps."miow"."0.2.1"."ws2_32_sys"}" deps)
    ]);
  };
  features_.miow."0.2.1" = deps: f: updateFeatures f (rec {
    kernel32_sys."${deps.miow."0.2.1".kernel32_sys}".default = true;
    miow."0.2.1".default = (f.miow."0.2.1".default or true);
    net2."${deps.miow."0.2.1".net2}".default = (f.net2."${deps.miow."0.2.1".net2}".default or false);
    winapi."${deps.miow."0.2.1".winapi}".default = true;
    ws2_32_sys."${deps.miow."0.2.1".ws2_32_sys}".default = true;
  }) [
    (features_.kernel32_sys."${deps."miow"."0.2.1"."kernel32_sys"}" deps)
    (features_.net2."${deps."miow"."0.2.1"."net2"}" deps)
    (features_.winapi."${deps."miow"."0.2.1"."winapi"}" deps)
    (features_.ws2_32_sys."${deps."miow"."0.2.1"."ws2_32_sys"}" deps)
  ];


  crates.net2."0.2.33" = deps: { features?(features_.net2."0.2.33" deps {}) }: buildRustCrate {
    crateName = "net2";
    version = "0.2.33";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "1qnmajafgybj5wyxz9iffa8x5wgbwd2znfklmhqj7vl6lw1m65mq";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."net2"."0.2.33"."cfg_if"}" deps)
    ])
      ++ (if kernel == "redox" || (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."net2"."0.2.33"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."net2"."0.2.33"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."net2"."0.2.33" or {});
  };
  features_.net2."0.2.33" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.net2."0.2.33".cfg_if}".default = true;
    libc."${deps.net2."0.2.33".libc}".default = true;
    net2 = fold recursiveUpdate {} [
      { "0.2.33".default = (f.net2."0.2.33".default or true); }
      { "0.2.33".duration =
        (f.net2."0.2.33".duration or false) ||
        (f.net2."0.2.33".default or false) ||
        (net2."0.2.33"."default" or false); }
    ];
    winapi = fold recursiveUpdate {} [
      { "${deps.net2."0.2.33".winapi}"."handleapi" = true; }
      { "${deps.net2."0.2.33".winapi}"."winsock2" = true; }
      { "${deps.net2."0.2.33".winapi}"."ws2def" = true; }
      { "${deps.net2."0.2.33".winapi}"."ws2ipdef" = true; }
      { "${deps.net2."0.2.33".winapi}"."ws2tcpip" = true; }
      { "${deps.net2."0.2.33".winapi}".default = true; }
    ];
  }) [
    (features_.cfg_if."${deps."net2"."0.2.33"."cfg_if"}" deps)
    (features_.libc."${deps."net2"."0.2.33"."libc"}" deps)
    (features_.winapi."${deps."net2"."0.2.33"."winapi"}" deps)
  ];


  crates.nodrop."0.1.12" = deps: { features?(features_.nodrop."0.1.12" deps {}) }: buildRustCrate {
    crateName = "nodrop";
    version = "0.1.12";
    authors = [ "bluss" ];
    sha256 = "1b9rxvdg8061gxjc239l9slndf0ds3m6fy2sf3gs8f9kknqgl49d";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."nodrop"."0.1.12" or {});
  };
  features_.nodrop."0.1.12" = deps: f: updateFeatures f (rec {
    nodrop = fold recursiveUpdate {} [
      { "0.1.12".default = (f.nodrop."0.1.12".default or true); }
      { "0.1.12".nodrop-union =
        (f.nodrop."0.1.12".nodrop-union or false) ||
        (f.nodrop."0.1.12".use_union or false) ||
        (nodrop."0.1.12"."use_union" or false); }
      { "0.1.12".std =
        (f.nodrop."0.1.12".std or false) ||
        (f.nodrop."0.1.12".default or false) ||
        (nodrop."0.1.12"."default" or false); }
    ];
  }) [];


  crates.num_cpus."1.8.0" = deps: { features?(features_.num_cpus."1.8.0" deps {}) }: buildRustCrate {
    crateName = "num_cpus";
    version = "1.8.0";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "1y6qnd9r8ga6y8mvlabdrr73nc8cshjjlzbvnanzyj9b8zzkfwk2";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."num_cpus"."1.8.0"."libc"}" deps)
    ]);
  };
  features_.num_cpus."1.8.0" = deps: f: updateFeatures f (rec {
    libc."${deps.num_cpus."1.8.0".libc}".default = true;
    num_cpus."1.8.0".default = (f.num_cpus."1.8.0".default or true);
  }) [
    (features_.libc."${deps."num_cpus"."1.8.0"."libc"}" deps)
  ];


  crates.owning_ref."0.3.3" = deps: { features?(features_.owning_ref."0.3.3" deps {}) }: buildRustCrate {
    crateName = "owning_ref";
    version = "0.3.3";
    authors = [ "Marvin Löbel <loebel.marvin@gmail.com>" ];
    sha256 = "13ivn0ydc0hf957ix0f5si9nnplzzykbr70hni1qz9m19i9kvmrh";
    dependencies = mapFeatures features ([
      (crates."stable_deref_trait"."${deps."owning_ref"."0.3.3"."stable_deref_trait"}" deps)
    ]);
  };
  features_.owning_ref."0.3.3" = deps: f: updateFeatures f (rec {
    owning_ref."0.3.3".default = (f.owning_ref."0.3.3".default or true);
    stable_deref_trait."${deps.owning_ref."0.3.3".stable_deref_trait}".default = true;
  }) [
    (features_.stable_deref_trait."${deps."owning_ref"."0.3.3"."stable_deref_trait"}" deps)
  ];


  crates.parking_lot."0.6.4" = deps: { features?(features_.parking_lot."0.6.4" deps {}) }: buildRustCrate {
    crateName = "parking_lot";
    version = "0.6.4";
    authors = [ "Amanieu d'Antras <amanieu@gmail.com>" ];
    sha256 = "0qwfysx8zfkj72sfcrqvd6pp7lgjmklyixsi3y0g6xjspw876rax";
    dependencies = mapFeatures features ([
      (crates."lock_api"."${deps."parking_lot"."0.6.4"."lock_api"}" deps)
      (crates."parking_lot_core"."${deps."parking_lot"."0.6.4"."parking_lot_core"}" deps)
    ]);
    features = mkFeatures (features."parking_lot"."0.6.4" or {});
  };
  features_.parking_lot."0.6.4" = deps: f: updateFeatures f (rec {
    lock_api = fold recursiveUpdate {} [
      { "${deps.parking_lot."0.6.4".lock_api}".default = true; }
      { "0.1.4".nightly =
        (f.lock_api."0.1.4".nightly or false) ||
        (parking_lot."0.6.4"."nightly" or false) ||
        (f."parking_lot"."0.6.4"."nightly" or false); }
      { "0.1.4".owning_ref =
        (f.lock_api."0.1.4".owning_ref or false) ||
        (parking_lot."0.6.4"."owning_ref" or false) ||
        (f."parking_lot"."0.6.4"."owning_ref" or false); }
    ];
    parking_lot = fold recursiveUpdate {} [
      { "0.6.4".default = (f.parking_lot."0.6.4".default or true); }
      { "0.6.4".owning_ref =
        (f.parking_lot."0.6.4".owning_ref or false) ||
        (f.parking_lot."0.6.4".default or false) ||
        (parking_lot."0.6.4"."default" or false); }
    ];
    parking_lot_core = fold recursiveUpdate {} [
      { "${deps.parking_lot."0.6.4".parking_lot_core}".default = true; }
      { "0.3.1".deadlock_detection =
        (f.parking_lot_core."0.3.1".deadlock_detection or false) ||
        (parking_lot."0.6.4"."deadlock_detection" or false) ||
        (f."parking_lot"."0.6.4"."deadlock_detection" or false); }
      { "0.3.1".nightly =
        (f.parking_lot_core."0.3.1".nightly or false) ||
        (parking_lot."0.6.4"."nightly" or false) ||
        (f."parking_lot"."0.6.4"."nightly" or false); }
    ];
  }) [
    (features_.lock_api."${deps."parking_lot"."0.6.4"."lock_api"}" deps)
    (features_.parking_lot_core."${deps."parking_lot"."0.6.4"."parking_lot_core"}" deps)
  ];


  crates.parking_lot_core."0.3.1" = deps: { features?(features_.parking_lot_core."0.3.1" deps {}) }: buildRustCrate {
    crateName = "parking_lot_core";
    version = "0.3.1";
    authors = [ "Amanieu d'Antras <amanieu@gmail.com>" ];
    sha256 = "0h5p7dys8cx9y6ii4i57ampf7qdr8zmkpn543kd3h7nkhml8bw72";
    dependencies = mapFeatures features ([
      (crates."rand"."${deps."parking_lot_core"."0.3.1"."rand"}" deps)
      (crates."smallvec"."${deps."parking_lot_core"."0.3.1"."smallvec"}" deps)
    ])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."parking_lot_core"."0.3.1"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."parking_lot_core"."0.3.1"."winapi"}" deps)
    ]) else []);

    buildDependencies = mapFeatures features ([
      (crates."rustc_version"."${deps."parking_lot_core"."0.3.1"."rustc_version"}" deps)
    ]);
    features = mkFeatures (features."parking_lot_core"."0.3.1" or {});
  };
  features_.parking_lot_core."0.3.1" = deps: f: updateFeatures f (rec {
    libc."${deps.parking_lot_core."0.3.1".libc}".default = true;
    parking_lot_core = fold recursiveUpdate {} [
      { "0.3.1".backtrace =
        (f.parking_lot_core."0.3.1".backtrace or false) ||
        (f.parking_lot_core."0.3.1".deadlock_detection or false) ||
        (parking_lot_core."0.3.1"."deadlock_detection" or false); }
      { "0.3.1".default = (f.parking_lot_core."0.3.1".default or true); }
      { "0.3.1".petgraph =
        (f.parking_lot_core."0.3.1".petgraph or false) ||
        (f.parking_lot_core."0.3.1".deadlock_detection or false) ||
        (parking_lot_core."0.3.1"."deadlock_detection" or false); }
      { "0.3.1".thread-id =
        (f.parking_lot_core."0.3.1".thread-id or false) ||
        (f.parking_lot_core."0.3.1".deadlock_detection or false) ||
        (parking_lot_core."0.3.1"."deadlock_detection" or false); }
    ];
    rand."${deps.parking_lot_core."0.3.1".rand}".default = true;
    rustc_version."${deps.parking_lot_core."0.3.1".rustc_version}".default = true;
    smallvec."${deps.parking_lot_core."0.3.1".smallvec}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.parking_lot_core."0.3.1".winapi}"."errhandlingapi" = true; }
      { "${deps.parking_lot_core."0.3.1".winapi}"."handleapi" = true; }
      { "${deps.parking_lot_core."0.3.1".winapi}"."minwindef" = true; }
      { "${deps.parking_lot_core."0.3.1".winapi}"."ntstatus" = true; }
      { "${deps.parking_lot_core."0.3.1".winapi}"."winbase" = true; }
      { "${deps.parking_lot_core."0.3.1".winapi}"."winerror" = true; }
      { "${deps.parking_lot_core."0.3.1".winapi}"."winnt" = true; }
      { "${deps.parking_lot_core."0.3.1".winapi}".default = true; }
    ];
  }) [
    (features_.rand."${deps."parking_lot_core"."0.3.1"."rand"}" deps)
    (features_.smallvec."${deps."parking_lot_core"."0.3.1"."smallvec"}" deps)
    (features_.rustc_version."${deps."parking_lot_core"."0.3.1"."rustc_version"}" deps)
    (features_.libc."${deps."parking_lot_core"."0.3.1"."libc"}" deps)
    (features_.winapi."${deps."parking_lot_core"."0.3.1"."winapi"}" deps)
  ];


  crates.percent_encoding."1.0.1" = deps: { features?(features_.percent_encoding."1.0.1" deps {}) }: buildRustCrate {
    crateName = "percent-encoding";
    version = "1.0.1";
    authors = [ "The rust-url developers" ];
    sha256 = "04ahrp7aw4ip7fmadb0bknybmkfav0kk0gw4ps3ydq5w6hr0ib5i";
    libPath = "lib.rs";
  };
  features_.percent_encoding."1.0.1" = deps: f: updateFeatures f (rec {
    percent_encoding."1.0.1".default = (f.percent_encoding."1.0.1".default or true);
  }) [];


  crates.proc_macro2."0.4.20" = deps: { features?(features_.proc_macro2."0.4.20" deps {}) }: buildRustCrate {
    crateName = "proc-macro2";
    version = "0.4.20";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "0yr74b00d3wzg21kjvfln7vzzvf9aghbaff4c747i3grbd997ys2";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."unicode_xid"."${deps."proc_macro2"."0.4.20"."unicode_xid"}" deps)
    ]);
    features = mkFeatures (features."proc_macro2"."0.4.20" or {});
  };
  features_.proc_macro2."0.4.20" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "0.4.20".default = (f.proc_macro2."0.4.20".default or true); }
      { "0.4.20".proc-macro =
        (f.proc_macro2."0.4.20".proc-macro or false) ||
        (f.proc_macro2."0.4.20".default or false) ||
        (proc_macro2."0.4.20"."default" or false) ||
        (f.proc_macro2."0.4.20".nightly or false) ||
        (proc_macro2."0.4.20"."nightly" or false); }
    ];
    unicode_xid."${deps.proc_macro2."0.4.20".unicode_xid}".default = true;
  }) [
    (features_.unicode_xid."${deps."proc_macro2"."0.4.20"."unicode_xid"}" deps)
  ];


  crates.quick_error."1.2.2" = deps: { features?(features_.quick_error."1.2.2" deps {}) }: buildRustCrate {
    crateName = "quick-error";
    version = "1.2.2";
    authors = [ "Paul Colomiets <paul@colomiets.name>" "Colin Kiegel <kiegel@gmx.de>" ];
    sha256 = "192a3adc5phgpibgqblsdx1b421l5yg9bjbmv552qqq9f37h60k5";
  };
  features_.quick_error."1.2.2" = deps: f: updateFeatures f (rec {
    quick_error."1.2.2".default = (f.quick_error."1.2.2".default or true);
  }) [];


  crates.quickcheck."0.7.1" = deps: { features?(features_.quickcheck."0.7.1" deps {}) }: buildRustCrate {
    crateName = "quickcheck";
    version = "0.7.1";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "0rl5lfg9xpjn6d9xk5wz2ihj4zx0qlfk530jr4ryc7vcspx8knbb";
    dependencies = mapFeatures features ([
      (crates."rand"."${deps."quickcheck"."0.7.1"."rand"}" deps)
      (crates."rand_core"."${deps."quickcheck"."0.7.1"."rand_core"}" deps)
    ]
      ++ (if features.quickcheck."0.7.1".env_logger or false then [ (crates.env_logger."0.5.13" deps) ] else [])
      ++ (if features.quickcheck."0.7.1".log or false then [ (crates.log."0.4.5" deps) ] else []));
    features = mkFeatures (features."quickcheck"."0.7.1" or {});
  };
  features_.quickcheck."0.7.1" = deps: f: updateFeatures f (rec {
    env_logger = fold recursiveUpdate {} [
      { "${deps.quickcheck."0.7.1".env_logger}".default = (f.env_logger."${deps.quickcheck."0.7.1".env_logger}".default or false); }
      { "0.5.13".regex =
        (f.env_logger."0.5.13".regex or false) ||
        (quickcheck."0.7.1"."regex" or false) ||
        (f."quickcheck"."0.7.1"."regex" or false); }
    ];
    log."${deps.quickcheck."0.7.1".log}".default = true;
    quickcheck = fold recursiveUpdate {} [
      { "0.7.1".default = (f.quickcheck."0.7.1".default or true); }
      { "0.7.1".env_logger =
        (f.quickcheck."0.7.1".env_logger or false) ||
        (f.quickcheck."0.7.1".use_logging or false) ||
        (quickcheck."0.7.1"."use_logging" or false); }
      { "0.7.1".log =
        (f.quickcheck."0.7.1".log or false) ||
        (f.quickcheck."0.7.1".use_logging or false) ||
        (quickcheck."0.7.1"."use_logging" or false); }
      { "0.7.1".regex =
        (f.quickcheck."0.7.1".regex or false) ||
        (f.quickcheck."0.7.1".default or false) ||
        (quickcheck."0.7.1"."default" or false); }
      { "0.7.1".use_logging =
        (f.quickcheck."0.7.1".use_logging or false) ||
        (f.quickcheck."0.7.1".default or false) ||
        (quickcheck."0.7.1"."default" or false); }
    ];
    rand = fold recursiveUpdate {} [
      { "${deps.quickcheck."0.7.1".rand}".default = true; }
      { "0.5.5".i128_support =
        (f.rand."0.5.5".i128_support or false) ||
        (quickcheck."0.7.1"."i128" or false) ||
        (f."quickcheck"."0.7.1"."i128" or false); }
    ];
    rand_core."${deps.quickcheck."0.7.1".rand_core}".default = true;
  }) [
    (features_.env_logger."${deps."quickcheck"."0.7.1"."env_logger"}" deps)
    (features_.log."${deps."quickcheck"."0.7.1"."log"}" deps)
    (features_.rand."${deps."quickcheck"."0.7.1"."rand"}" deps)
    (features_.rand_core."${deps."quickcheck"."0.7.1"."rand_core"}" deps)
  ];


  crates.quote."0.6.8" = deps: { features?(features_.quote."0.6.8" deps {}) }: buildRustCrate {
    crateName = "quote";
    version = "0.6.8";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "0dq6j23w6pmc4l6v490arixdwypy0b82z76nrzaingqhqri4p3mh";
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."quote"."0.6.8"."proc_macro2"}" deps)
    ]);
    features = mkFeatures (features."quote"."0.6.8" or {});
  };
  features_.quote."0.6.8" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "${deps.quote."0.6.8".proc_macro2}".default = (f.proc_macro2."${deps.quote."0.6.8".proc_macro2}".default or false); }
      { "0.4.20".proc-macro =
        (f.proc_macro2."0.4.20".proc-macro or false) ||
        (quote."0.6.8"."proc-macro" or false) ||
        (f."quote"."0.6.8"."proc-macro" or false); }
    ];
    quote = fold recursiveUpdate {} [
      { "0.6.8".default = (f.quote."0.6.8".default or true); }
      { "0.6.8".proc-macro =
        (f.quote."0.6.8".proc-macro or false) ||
        (f.quote."0.6.8".default or false) ||
        (quote."0.6.8"."default" or false); }
    ];
  }) [
    (features_.proc_macro2."${deps."quote"."0.6.8"."proc_macro2"}" deps)
  ];


  crates.rand."0.3.22" = deps: { features?(features_.rand."0.3.22" deps {}) }: buildRustCrate {
    crateName = "rand";
    version = "0.3.22";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0wrj12acx7l4hr7ag3nz8b50yhp8ancyq988bzmnnsxln67rsys0";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."rand"."0.3.22"."libc"}" deps)
      (crates."rand"."${deps."rand"."0.3.22"."rand"}" deps)
    ])
      ++ (if kernel == "fuchsia" then mapFeatures features ([
      (crates."fuchsia_zircon"."${deps."rand"."0.3.22"."fuchsia_zircon"}" deps)
    ]) else []);
    features = mkFeatures (features."rand"."0.3.22" or {});
  };
  features_.rand."0.3.22" = deps: f: updateFeatures f (rec {
    fuchsia_zircon."${deps.rand."0.3.22".fuchsia_zircon}".default = true;
    libc."${deps.rand."0.3.22".libc}".default = true;
    rand = fold recursiveUpdate {} [
      { "${deps.rand."0.3.22".rand}".default = true; }
      { "0.3.22".default = (f.rand."0.3.22".default or true); }
      { "0.3.22".i128_support =
        (f.rand."0.3.22".i128_support or false) ||
        (f.rand."0.3.22".nightly or false) ||
        (rand."0.3.22"."nightly" or false); }
    ];
  }) [
    (features_.libc."${deps."rand"."0.3.22"."libc"}" deps)
    (features_.rand."${deps."rand"."0.3.22"."rand"}" deps)
    (features_.fuchsia_zircon."${deps."rand"."0.3.22"."fuchsia_zircon"}" deps)
  ];


  crates.rand."0.4.3" = deps: { features?(features_.rand."0.4.3" deps {}) }: buildRustCrate {
    crateName = "rand";
    version = "0.4.3";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1644wri45l147822xy7dgdm4k7myxzs66cb795ga0x7dan11ci4f";
    dependencies = (if kernel == "fuchsia" then mapFeatures features ([
      (crates."fuchsia_zircon"."${deps."rand"."0.4.3"."fuchsia_zircon"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
    ]
      ++ (if features.rand."0.4.3".libc or false then [ (crates.libc."0.2.43" deps) ] else [])) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."rand"."0.4.3"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."rand"."0.4.3" or {});
  };
  features_.rand."0.4.3" = deps: f: updateFeatures f (rec {
    fuchsia_zircon."${deps.rand."0.4.3".fuchsia_zircon}".default = true;
    libc."${deps.rand."0.4.3".libc}".default = true;
    rand = fold recursiveUpdate {} [
      { "0.4.3".default = (f.rand."0.4.3".default or true); }
      { "0.4.3".i128_support =
        (f.rand."0.4.3".i128_support or false) ||
        (f.rand."0.4.3".nightly or false) ||
        (rand."0.4.3"."nightly" or false); }
      { "0.4.3".libc =
        (f.rand."0.4.3".libc or false) ||
        (f.rand."0.4.3".std or false) ||
        (rand."0.4.3"."std" or false); }
      { "0.4.3".std =
        (f.rand."0.4.3".std or false) ||
        (f.rand."0.4.3".default or false) ||
        (rand."0.4.3"."default" or false); }
    ];
    winapi = fold recursiveUpdate {} [
      { "${deps.rand."0.4.3".winapi}"."minwindef" = true; }
      { "${deps.rand."0.4.3".winapi}"."ntsecapi" = true; }
      { "${deps.rand."0.4.3".winapi}"."profileapi" = true; }
      { "${deps.rand."0.4.3".winapi}"."winnt" = true; }
      { "${deps.rand."0.4.3".winapi}".default = true; }
    ];
  }) [
    (features_.fuchsia_zircon."${deps."rand"."0.4.3"."fuchsia_zircon"}" deps)
    (features_.libc."${deps."rand"."0.4.3"."libc"}" deps)
    (features_.winapi."${deps."rand"."0.4.3"."winapi"}" deps)
  ];


  crates.rand."0.5.5" = deps: { features?(features_.rand."0.5.5" deps {}) }: buildRustCrate {
    crateName = "rand";
    version = "0.5.5";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0d7pnsh57qxhz1ghrzk113ddkn13kf2g758ffnbxq4nhwjfzhlc9";
    dependencies = mapFeatures features ([
      (crates."rand_core"."${deps."rand"."0.5.5"."rand_core"}" deps)
    ])
      ++ (if kernel == "cloudabi" then mapFeatures features ([
    ]
      ++ (if features.rand."0.5.5".cloudabi or false then [ (crates.cloudabi."0.0.3" deps) ] else [])) else [])
      ++ (if kernel == "fuchsia" then mapFeatures features ([
    ]
      ++ (if features.rand."0.5.5".fuchsia-zircon or false then [ (crates.fuchsia_zircon."0.3.3" deps) ] else [])) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
    ]
      ++ (if features.rand."0.5.5".libc or false then [ (crates.libc."0.2.43" deps) ] else [])) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
    ]
      ++ (if features.rand."0.5.5".winapi or false then [ (crates.winapi."0.3.6" deps) ] else [])) else [])
      ++ (if kernel == "wasm32-unknown-unknown" then mapFeatures features ([
]) else []);
    features = mkFeatures (features."rand"."0.5.5" or {});
  };
  features_.rand."0.5.5" = deps: f: updateFeatures f (rec {
    cloudabi."${deps.rand."0.5.5".cloudabi}".default = true;
    fuchsia_zircon."${deps.rand."0.5.5".fuchsia_zircon}".default = true;
    libc."${deps.rand."0.5.5".libc}".default = true;
    rand = fold recursiveUpdate {} [
      { "0.5.5".alloc =
        (f.rand."0.5.5".alloc or false) ||
        (f.rand."0.5.5".std or false) ||
        (rand."0.5.5"."std" or false); }
      { "0.5.5".cloudabi =
        (f.rand."0.5.5".cloudabi or false) ||
        (f.rand."0.5.5".std or false) ||
        (rand."0.5.5"."std" or false); }
      { "0.5.5".default = (f.rand."0.5.5".default or true); }
      { "0.5.5".fuchsia-zircon =
        (f.rand."0.5.5".fuchsia-zircon or false) ||
        (f.rand."0.5.5".std or false) ||
        (rand."0.5.5"."std" or false); }
      { "0.5.5".i128_support =
        (f.rand."0.5.5".i128_support or false) ||
        (f.rand."0.5.5".nightly or false) ||
        (rand."0.5.5"."nightly" or false); }
      { "0.5.5".libc =
        (f.rand."0.5.5".libc or false) ||
        (f.rand."0.5.5".std or false) ||
        (rand."0.5.5"."std" or false); }
      { "0.5.5".serde =
        (f.rand."0.5.5".serde or false) ||
        (f.rand."0.5.5".serde1 or false) ||
        (rand."0.5.5"."serde1" or false); }
      { "0.5.5".serde_derive =
        (f.rand."0.5.5".serde_derive or false) ||
        (f.rand."0.5.5".serde1 or false) ||
        (rand."0.5.5"."serde1" or false); }
      { "0.5.5".std =
        (f.rand."0.5.5".std or false) ||
        (f.rand."0.5.5".default or false) ||
        (rand."0.5.5"."default" or false); }
      { "0.5.5".winapi =
        (f.rand."0.5.5".winapi or false) ||
        (f.rand."0.5.5".std or false) ||
        (rand."0.5.5"."std" or false); }
    ];
    rand_core = fold recursiveUpdate {} [
      { "${deps.rand."0.5.5".rand_core}".default = (f.rand_core."${deps.rand."0.5.5".rand_core}".default or false); }
      { "0.2.2".alloc =
        (f.rand_core."0.2.2".alloc or false) ||
        (rand."0.5.5"."alloc" or false) ||
        (f."rand"."0.5.5"."alloc" or false); }
      { "0.2.2".serde1 =
        (f.rand_core."0.2.2".serde1 or false) ||
        (rand."0.5.5"."serde1" or false) ||
        (f."rand"."0.5.5"."serde1" or false); }
      { "0.2.2".std =
        (f.rand_core."0.2.2".std or false) ||
        (rand."0.5.5"."std" or false) ||
        (f."rand"."0.5.5"."std" or false); }
    ];
    winapi = fold recursiveUpdate {} [
      { "${deps.rand."0.5.5".winapi}"."minwindef" = true; }
      { "${deps.rand."0.5.5".winapi}"."ntsecapi" = true; }
      { "${deps.rand."0.5.5".winapi}"."profileapi" = true; }
      { "${deps.rand."0.5.5".winapi}"."winnt" = true; }
      { "${deps.rand."0.5.5".winapi}".default = true; }
    ];
  }) [
    (features_.rand_core."${deps."rand"."0.5.5"."rand_core"}" deps)
    (features_.cloudabi."${deps."rand"."0.5.5"."cloudabi"}" deps)
    (features_.fuchsia_zircon."${deps."rand"."0.5.5"."fuchsia_zircon"}" deps)
    (features_.libc."${deps."rand"."0.5.5"."libc"}" deps)
    (features_.winapi."${deps."rand"."0.5.5"."winapi"}" deps)
  ];


  crates.rand_core."0.2.2" = deps: { features?(features_.rand_core."0.2.2" deps {}) }: buildRustCrate {
    crateName = "rand_core";
    version = "0.2.2";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1cxnaxmsirz2wxsajsjkd1wk6lqfqbcprqkha4bq3didznrl22sc";
    dependencies = mapFeatures features ([
      (crates."rand_core"."${deps."rand_core"."0.2.2"."rand_core"}" deps)
    ]);
    features = mkFeatures (features."rand_core"."0.2.2" or {});
  };
  features_.rand_core."0.2.2" = deps: f: updateFeatures f (rec {
    rand_core = fold recursiveUpdate {} [
      { "${deps.rand_core."0.2.2".rand_core}".default = (f.rand_core."${deps.rand_core."0.2.2".rand_core}".default or false); }
      { "0.2.2".default = (f.rand_core."0.2.2".default or true); }
      { "0.3.0".alloc =
        (f.rand_core."0.3.0".alloc or false) ||
        (rand_core."0.2.2"."alloc" or false) ||
        (f."rand_core"."0.2.2"."alloc" or false); }
      { "0.3.0".serde1 =
        (f.rand_core."0.3.0".serde1 or false) ||
        (rand_core."0.2.2"."serde1" or false) ||
        (f."rand_core"."0.2.2"."serde1" or false); }
      { "0.3.0".std =
        (f.rand_core."0.3.0".std or false) ||
        (rand_core."0.2.2"."std" or false) ||
        (f."rand_core"."0.2.2"."std" or false); }
    ];
  }) [
    (features_.rand_core."${deps."rand_core"."0.2.2"."rand_core"}" deps)
  ];


  crates.rand_core."0.3.0" = deps: { features?(features_.rand_core."0.3.0" deps {}) }: buildRustCrate {
    crateName = "rand_core";
    version = "0.3.0";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1vafw316apjys9va3j987s02djhqp7y21v671v3ix0p5j9bjq339";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."rand_core"."0.3.0" or {});
  };
  features_.rand_core."0.3.0" = deps: f: updateFeatures f (rec {
    rand_core = fold recursiveUpdate {} [
      { "0.3.0".alloc =
        (f.rand_core."0.3.0".alloc or false) ||
        (f.rand_core."0.3.0".std or false) ||
        (rand_core."0.3.0"."std" or false); }
      { "0.3.0".default = (f.rand_core."0.3.0".default or true); }
      { "0.3.0".serde =
        (f.rand_core."0.3.0".serde or false) ||
        (f.rand_core."0.3.0".serde1 or false) ||
        (rand_core."0.3.0"."serde1" or false); }
      { "0.3.0".serde_derive =
        (f.rand_core."0.3.0".serde_derive or false) ||
        (f.rand_core."0.3.0".serde1 or false) ||
        (rand_core."0.3.0"."serde1" or false); }
      { "0.3.0".std =
        (f.rand_core."0.3.0".std or false) ||
        (f.rand_core."0.3.0".default or false) ||
        (rand_core."0.3.0"."default" or false); }
    ];
  }) [];


  crates.redox_syscall."0.1.40" = deps: { features?(features_.redox_syscall."0.1.40" deps {}) }: buildRustCrate {
    crateName = "redox_syscall";
    version = "0.1.40";
    authors = [ "Jeremy Soller <jackpot51@gmail.com>" ];
    sha256 = "132rnhrq49l3z7gjrwj2zfadgw6q0355s6a7id7x7c0d7sk72611";
    libName = "syscall";
  };
  features_.redox_syscall."0.1.40" = deps: f: updateFeatures f (rec {
    redox_syscall."0.1.40".default = (f.redox_syscall."0.1.40".default or true);
  }) [];


  crates.redox_termios."0.1.1" = deps: { features?(features_.redox_termios."0.1.1" deps {}) }: buildRustCrate {
    crateName = "redox_termios";
    version = "0.1.1";
    authors = [ "Jeremy Soller <jackpot51@gmail.com>" ];
    sha256 = "04s6yyzjca552hdaqlvqhp3vw0zqbc304md5czyd3axh56iry8wh";
    libPath = "src/lib.rs";
    dependencies = mapFeatures features ([
      (crates."redox_syscall"."${deps."redox_termios"."0.1.1"."redox_syscall"}" deps)
    ]);
  };
  features_.redox_termios."0.1.1" = deps: f: updateFeatures f (rec {
    redox_syscall."${deps.redox_termios."0.1.1".redox_syscall}".default = true;
    redox_termios."0.1.1".default = (f.redox_termios."0.1.1".default or true);
  }) [
    (features_.redox_syscall."${deps."redox_termios"."0.1.1"."redox_syscall"}" deps)
  ];


  crates.regex."1.0.5" = deps: { features?(features_.regex."1.0.5" deps {}) }: buildRustCrate {
    crateName = "regex";
    version = "1.0.5";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1nb4dva9lhb3v76bdds9qcxldb2xy998sdraqnqaqdr6axfsfp02";
    dependencies = mapFeatures features ([
      (crates."aho_corasick"."${deps."regex"."1.0.5"."aho_corasick"}" deps)
      (crates."memchr"."${deps."regex"."1.0.5"."memchr"}" deps)
      (crates."regex_syntax"."${deps."regex"."1.0.5"."regex_syntax"}" deps)
      (crates."thread_local"."${deps."regex"."1.0.5"."thread_local"}" deps)
      (crates."utf8_ranges"."${deps."regex"."1.0.5"."utf8_ranges"}" deps)
    ]);
    features = mkFeatures (features."regex"."1.0.5" or {});
  };
  features_.regex."1.0.5" = deps: f: updateFeatures f (rec {
    aho_corasick."${deps.regex."1.0.5".aho_corasick}".default = true;
    memchr."${deps.regex."1.0.5".memchr}".default = true;
    regex = fold recursiveUpdate {} [
      { "1.0.5".default = (f.regex."1.0.5".default or true); }
      { "1.0.5".pattern =
        (f.regex."1.0.5".pattern or false) ||
        (f.regex."1.0.5".unstable or false) ||
        (regex."1.0.5"."unstable" or false); }
      { "1.0.5".use_std =
        (f.regex."1.0.5".use_std or false) ||
        (f.regex."1.0.5".default or false) ||
        (regex."1.0.5"."default" or false); }
    ];
    regex_syntax."${deps.regex."1.0.5".regex_syntax}".default = true;
    thread_local."${deps.regex."1.0.5".thread_local}".default = true;
    utf8_ranges."${deps.regex."1.0.5".utf8_ranges}".default = true;
  }) [
    (features_.aho_corasick."${deps."regex"."1.0.5"."aho_corasick"}" deps)
    (features_.memchr."${deps."regex"."1.0.5"."memchr"}" deps)
    (features_.regex_syntax."${deps."regex"."1.0.5"."regex_syntax"}" deps)
    (features_.thread_local."${deps."regex"."1.0.5"."thread_local"}" deps)
    (features_.utf8_ranges."${deps."regex"."1.0.5"."utf8_ranges"}" deps)
  ];


  crates.regex_syntax."0.6.2" = deps: { features?(features_.regex_syntax."0.6.2" deps {}) }: buildRustCrate {
    crateName = "regex-syntax";
    version = "0.6.2";
    authors = [ "The Rust Project Developers" ];
    sha256 = "109426mj7nhwr6szdzbcvn1a8g5zy52f9maqxjd9agm8wg87ylyw";
    dependencies = mapFeatures features ([
      (crates."ucd_util"."${deps."regex_syntax"."0.6.2"."ucd_util"}" deps)
    ]);
  };
  features_.regex_syntax."0.6.2" = deps: f: updateFeatures f (rec {
    regex_syntax."0.6.2".default = (f.regex_syntax."0.6.2".default or true);
    ucd_util."${deps.regex_syntax."0.6.2".ucd_util}".default = true;
  }) [
    (features_.ucd_util."${deps."regex_syntax"."0.6.2"."ucd_util"}" deps)
  ];


  crates.relay."0.1.1" = deps: { features?(features_.relay."0.1.1" deps {}) }: buildRustCrate {
    crateName = "relay";
    version = "0.1.1";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "16csfaslbmj25iaxs88p8wcfh2zfpkh9isg9adid0nxjxvknh07r";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."relay"."0.1.1"."futures"}" deps)
    ]);
  };
  features_.relay."0.1.1" = deps: f: updateFeatures f (rec {
    futures."${deps.relay."0.1.1".futures}".default = true;
    relay."0.1.1".default = (f.relay."0.1.1".default or true);
  }) [
    (features_.futures."${deps."relay"."0.1.1"."futures"}" deps)
  ];


  crates.rustc_version."0.2.3" = deps: { features?(features_.rustc_version."0.2.3" deps {}) }: buildRustCrate {
    crateName = "rustc_version";
    version = "0.2.3";
    authors = [ "Marvin Löbel <loebel.marvin@gmail.com>" ];
    sha256 = "0rgwzbgs3i9fqjm1p4ra3n7frafmpwl29c8lw85kv1rxn7n2zaa7";
    dependencies = mapFeatures features ([
      (crates."semver"."${deps."rustc_version"."0.2.3"."semver"}" deps)
    ]);
  };
  features_.rustc_version."0.2.3" = deps: f: updateFeatures f (rec {
    rustc_version."0.2.3".default = (f.rustc_version."0.2.3".default or true);
    semver."${deps.rustc_version."0.2.3".semver}".default = true;
  }) [
    (features_.semver."${deps."rustc_version"."0.2.3"."semver"}" deps)
  ];


  crates.ryu."0.2.6" = deps: { features?(features_.ryu."0.2.6" deps {}) }: buildRustCrate {
    crateName = "ryu";
    version = "0.2.6";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1vdh6z4aysc9kiiqhl7vxkqz3fykcnp24kgfizshlwfsz2j0p9dr";
    build = "build.rs";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."ryu"."0.2.6" or {});
  };
  features_.ryu."0.2.6" = deps: f: updateFeatures f (rec {
    ryu."0.2.6".default = (f.ryu."0.2.6".default or true);
  }) [];


  crates.safemem."0.3.0" = deps: { features?(features_.safemem."0.3.0" deps {}) }: buildRustCrate {
    crateName = "safemem";
    version = "0.3.0";
    authors = [ "Austin Bonander <austin.bonander@gmail.com>" ];
    sha256 = "0pr39b468d05f6m7m4alsngmj5p7an8df21apsxbi57k0lmwrr18";
    features = mkFeatures (features."safemem"."0.3.0" or {});
  };
  features_.safemem."0.3.0" = deps: f: updateFeatures f (rec {
    safemem = fold recursiveUpdate {} [
      { "0.3.0".default = (f.safemem."0.3.0".default or true); }
      { "0.3.0".std =
        (f.safemem."0.3.0".std or false) ||
        (f.safemem."0.3.0".default or false) ||
        (safemem."0.3.0"."default" or false); }
    ];
  }) [];


  crates.scoped_tls."0.1.2" = deps: { features?(features_.scoped_tls."0.1.2" deps {}) }: buildRustCrate {
    crateName = "scoped-tls";
    version = "0.1.2";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "0nblksgki698cqsclsnd6f1pq4yy34350dn2slaah9dlmx9z5xla";
    features = mkFeatures (features."scoped_tls"."0.1.2" or {});
  };
  features_.scoped_tls."0.1.2" = deps: f: updateFeatures f (rec {
    scoped_tls."0.1.2".default = (f.scoped_tls."0.1.2".default or true);
  }) [];


  crates.scopeguard."0.3.3" = deps: { features?(features_.scopeguard."0.3.3" deps {}) }: buildRustCrate {
    crateName = "scopeguard";
    version = "0.3.3";
    authors = [ "bluss" ];
    sha256 = "0i1l013csrqzfz6c68pr5pi01hg5v5yahq8fsdmaxy6p8ygsjf3r";
    features = mkFeatures (features."scopeguard"."0.3.3" or {});
  };
  features_.scopeguard."0.3.3" = deps: f: updateFeatures f (rec {
    scopeguard = fold recursiveUpdate {} [
      { "0.3.3".default = (f.scopeguard."0.3.3".default or true); }
      { "0.3.3".use_std =
        (f.scopeguard."0.3.3".use_std or false) ||
        (f.scopeguard."0.3.3".default or false) ||
        (scopeguard."0.3.3"."default" or false); }
    ];
  }) [];


  crates.semver."0.9.0" = deps: { features?(features_.semver."0.9.0" deps {}) }: buildRustCrate {
    crateName = "semver";
    version = "0.9.0";
    authors = [ "Steve Klabnik <steve@steveklabnik.com>" "The Rust Project Developers" ];
    sha256 = "0azak2lb2wc36s3x15az886kck7rpnksrw14lalm157rg9sc9z63";
    dependencies = mapFeatures features ([
      (crates."semver_parser"."${deps."semver"."0.9.0"."semver_parser"}" deps)
    ]);
    features = mkFeatures (features."semver"."0.9.0" or {});
  };
  features_.semver."0.9.0" = deps: f: updateFeatures f (rec {
    semver = fold recursiveUpdate {} [
      { "0.9.0".default = (f.semver."0.9.0".default or true); }
      { "0.9.0".serde =
        (f.semver."0.9.0".serde or false) ||
        (f.semver."0.9.0".ci or false) ||
        (semver."0.9.0"."ci" or false); }
    ];
    semver_parser."${deps.semver."0.9.0".semver_parser}".default = true;
  }) [
    (features_.semver_parser."${deps."semver"."0.9.0"."semver_parser"}" deps)
  ];


  crates.semver_parser."0.7.0" = deps: { features?(features_.semver_parser."0.7.0" deps {}) }: buildRustCrate {
    crateName = "semver-parser";
    version = "0.7.0";
    authors = [ "Steve Klabnik <steve@steveklabnik.com>" ];
    sha256 = "1da66c8413yakx0y15k8c055yna5lyb6fr0fw9318kdwkrk5k12h";
  };
  features_.semver_parser."0.7.0" = deps: f: updateFeatures f (rec {
    semver_parser."0.7.0".default = (f.semver_parser."0.7.0".default or true);
  }) [];


  crates.serde."1.0.80" = deps: { features?(features_.serde."1.0.80" deps {}) }: buildRustCrate {
    crateName = "serde";
    version = "1.0.80";
    authors = [ "Erick Tryzelaar <erick.tryzelaar@gmail.com>" "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "0vyciw2qhrws4hz87pfnsjdfzzdw2sclxqxq394g3a219a2rdcxz";
    build = "build.rs";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."serde"."1.0.80" or {});
  };
  features_.serde."1.0.80" = deps: f: updateFeatures f (rec {
    serde = fold recursiveUpdate {} [
      { "1.0.80".default = (f.serde."1.0.80".default or true); }
      { "1.0.80".serde_derive =
        (f.serde."1.0.80".serde_derive or false) ||
        (f.serde."1.0.80".derive or false) ||
        (serde."1.0.80"."derive" or false); }
      { "1.0.80".std =
        (f.serde."1.0.80".std or false) ||
        (f.serde."1.0.80".default or false) ||
        (serde."1.0.80"."default" or false); }
      { "1.0.80".unstable =
        (f.serde."1.0.80".unstable or false) ||
        (f.serde."1.0.80".alloc or false) ||
        (serde."1.0.80"."alloc" or false); }
    ];
  }) [];


  crates.serde_derive."1.0.80" = deps: { features?(features_.serde_derive."1.0.80" deps {}) }: buildRustCrate {
    crateName = "serde_derive";
    version = "1.0.80";
    authors = [ "Erick Tryzelaar <erick.tryzelaar@gmail.com>" "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1akvzhbnnqhd92lfj7vp43scs1vdml7x27c82l5yh0kz7xf7jaky";
    procMacro = true;
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."serde_derive"."1.0.80"."proc_macro2"}" deps)
      (crates."quote"."${deps."serde_derive"."1.0.80"."quote"}" deps)
      (crates."syn"."${deps."serde_derive"."1.0.80"."syn"}" deps)
    ]);
    features = mkFeatures (features."serde_derive"."1.0.80" or {});
  };
  features_.serde_derive."1.0.80" = deps: f: updateFeatures f (rec {
    proc_macro2."${deps.serde_derive."1.0.80".proc_macro2}".default = true;
    quote."${deps.serde_derive."1.0.80".quote}".default = true;
    serde_derive."1.0.80".default = (f.serde_derive."1.0.80".default or true);
    syn = fold recursiveUpdate {} [
      { "${deps.serde_derive."1.0.80".syn}"."visit" = true; }
      { "${deps.serde_derive."1.0.80".syn}".default = true; }
    ];
  }) [
    (features_.proc_macro2."${deps."serde_derive"."1.0.80"."proc_macro2"}" deps)
    (features_.quote."${deps."serde_derive"."1.0.80"."quote"}" deps)
    (features_.syn."${deps."serde_derive"."1.0.80"."syn"}" deps)
  ];


  crates.serde_json."1.0.32" = deps: { features?(features_.serde_json."1.0.32" deps {}) }: buildRustCrate {
    crateName = "serde_json";
    version = "1.0.32";
    authors = [ "Erick Tryzelaar <erick.tryzelaar@gmail.com>" "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1dqkvizi02j1bs5c21kw20idf4aa5399g29ndwl6vkmmrqkr1gr0";
    dependencies = mapFeatures features ([
      (crates."itoa"."${deps."serde_json"."1.0.32"."itoa"}" deps)
      (crates."ryu"."${deps."serde_json"."1.0.32"."ryu"}" deps)
      (crates."serde"."${deps."serde_json"."1.0.32"."serde"}" deps)
    ]);
    features = mkFeatures (features."serde_json"."1.0.32" or {});
  };
  features_.serde_json."1.0.32" = deps: f: updateFeatures f (rec {
    itoa."${deps.serde_json."1.0.32".itoa}".default = true;
    ryu."${deps.serde_json."1.0.32".ryu}".default = true;
    serde."${deps.serde_json."1.0.32".serde}".default = true;
    serde_json = fold recursiveUpdate {} [
      { "1.0.32".default = (f.serde_json."1.0.32".default or true); }
      { "1.0.32".indexmap =
        (f.serde_json."1.0.32".indexmap or false) ||
        (f.serde_json."1.0.32".preserve_order or false) ||
        (serde_json."1.0.32"."preserve_order" or false); }
    ];
  }) [
    (features_.itoa."${deps."serde_json"."1.0.32"."itoa"}" deps)
    (features_.ryu."${deps."serde_json"."1.0.32"."ryu"}" deps)
    (features_.serde."${deps."serde_json"."1.0.32"."serde"}" deps)
  ];


  crates.serde_yaml."0.7.5" = deps: { features?(features_.serde_yaml."0.7.5" deps {}) }: buildRustCrate {
    crateName = "serde_yaml";
    version = "0.7.5";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1ai03b8gii88kziljn4ja3ayd6mc3zy0y8aq2wncxwkh0gd707gd";
    dependencies = mapFeatures features ([
      (crates."dtoa"."${deps."serde_yaml"."0.7.5"."dtoa"}" deps)
      (crates."linked_hash_map"."${deps."serde_yaml"."0.7.5"."linked_hash_map"}" deps)
      (crates."serde"."${deps."serde_yaml"."0.7.5"."serde"}" deps)
      (crates."yaml_rust"."${deps."serde_yaml"."0.7.5"."yaml_rust"}" deps)
    ]);
  };
  features_.serde_yaml."0.7.5" = deps: f: updateFeatures f (rec {
    dtoa."${deps.serde_yaml."0.7.5".dtoa}".default = true;
    linked_hash_map."${deps.serde_yaml."0.7.5".linked_hash_map}".default = true;
    serde."${deps.serde_yaml."0.7.5".serde}".default = true;
    serde_yaml."0.7.5".default = (f.serde_yaml."0.7.5".default or true);
    yaml_rust."${deps.serde_yaml."0.7.5".yaml_rust}".default = true;
  }) [
    (features_.dtoa."${deps."serde_yaml"."0.7.5"."dtoa"}" deps)
    (features_.linked_hash_map."${deps."serde_yaml"."0.7.5"."linked_hash_map"}" deps)
    (features_.serde."${deps."serde_yaml"."0.7.5"."serde"}" deps)
    (features_.yaml_rust."${deps."serde_yaml"."0.7.5"."yaml_rust"}" deps)
  ];


  crates.slab."0.3.0" = deps: { features?(features_.slab."0.3.0" deps {}) }: buildRustCrate {
    crateName = "slab";
    version = "0.3.0";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0y6lhjggksh57hyfd3l6p9wgv5nhvw9c6djrysq7jnalz8fih21k";
  };
  features_.slab."0.3.0" = deps: f: updateFeatures f (rec {
    slab."0.3.0".default = (f.slab."0.3.0".default or true);
  }) [];


  crates.slab."0.4.1" = deps: { features?(features_.slab."0.4.1" deps {}) }: buildRustCrate {
    crateName = "slab";
    version = "0.4.1";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0njmznhcjp4aiznybxm7wacnb4q49ch98wizyf4lpn3rg6sjrak4";
  };
  features_.slab."0.4.1" = deps: f: updateFeatures f (rec {
    slab."0.4.1".default = (f.slab."0.4.1".default or true);
  }) [];


  crates.smallvec."0.2.1" = deps: { features?(features_.smallvec."0.2.1" deps {}) }: buildRustCrate {
    crateName = "smallvec";
    version = "0.2.1";
    authors = [ "Simon Sapin <simon.sapin@exyr.org>" ];
    sha256 = "0rnsll9af52bpjngz0067dpm1ndqmh76i64a58fc118l4lvnjxw2";
    libPath = "lib.rs";
  };
  features_.smallvec."0.2.1" = deps: f: updateFeatures f (rec {
    smallvec."0.2.1".default = (f.smallvec."0.2.1".default or true);
  }) [];


  crates.smallvec."0.6.5" = deps: { features?(features_.smallvec."0.6.5" deps {}) }: buildRustCrate {
    crateName = "smallvec";
    version = "0.6.5";
    authors = [ "Simon Sapin <simon.sapin@exyr.org>" ];
    sha256 = "0jx49nb1c91936jaq30axc96wncdp2wdmspf1qc8iv85f1i44vvf";
    libPath = "lib.rs";
    dependencies = mapFeatures features ([
      (crates."unreachable"."${deps."smallvec"."0.6.5"."unreachable"}" deps)
    ]);
    features = mkFeatures (features."smallvec"."0.6.5" or {});
  };
  features_.smallvec."0.6.5" = deps: f: updateFeatures f (rec {
    smallvec = fold recursiveUpdate {} [
      { "0.6.5".default = (f.smallvec."0.6.5".default or true); }
      { "0.6.5".std =
        (f.smallvec."0.6.5".std or false) ||
        (f.smallvec."0.6.5".default or false) ||
        (smallvec."0.6.5"."default" or false); }
    ];
    unreachable."${deps.smallvec."0.6.5".unreachable}".default = true;
  }) [
    (features_.unreachable."${deps."smallvec"."0.6.5"."unreachable"}" deps)
  ];


  crates.stable_deref_trait."1.1.1" = deps: { features?(features_.stable_deref_trait."1.1.1" deps {}) }: buildRustCrate {
    crateName = "stable_deref_trait";
    version = "1.1.1";
    authors = [ "Robert Grosse <n210241048576@gmail.com>" ];
    sha256 = "1xy9slzslrzr31nlnw52sl1d820b09y61b7f13lqgsn8n7y0l4g8";
    features = mkFeatures (features."stable_deref_trait"."1.1.1" or {});
  };
  features_.stable_deref_trait."1.1.1" = deps: f: updateFeatures f (rec {
    stable_deref_trait = fold recursiveUpdate {} [
      { "1.1.1".default = (f.stable_deref_trait."1.1.1".default or true); }
      { "1.1.1".std =
        (f.stable_deref_trait."1.1.1".std or false) ||
        (f.stable_deref_trait."1.1.1".default or false) ||
        (stable_deref_trait."1.1.1"."default" or false); }
    ];
  }) [];


  crates.syn."0.15.13" = deps: { features?(features_.syn."0.15.13" deps {}) }: buildRustCrate {
    crateName = "syn";
    version = "0.15.13";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1zvnppl08f2njpkl3m10h221sdl4vsm7v6vyq63dxk16nn37b1bh";
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."syn"."0.15.13"."proc_macro2"}" deps)
      (crates."unicode_xid"."${deps."syn"."0.15.13"."unicode_xid"}" deps)
    ]
      ++ (if features.syn."0.15.13".quote or false then [ (crates.quote."0.6.8" deps) ] else []));
    features = mkFeatures (features."syn"."0.15.13" or {});
  };
  features_.syn."0.15.13" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "${deps.syn."0.15.13".proc_macro2}".default = (f.proc_macro2."${deps.syn."0.15.13".proc_macro2}".default or false); }
      { "0.4.20".proc-macro =
        (f.proc_macro2."0.4.20".proc-macro or false) ||
        (syn."0.15.13"."proc-macro" or false) ||
        (f."syn"."0.15.13"."proc-macro" or false); }
    ];
    quote = fold recursiveUpdate {} [
      { "${deps.syn."0.15.13".quote}".default = (f.quote."${deps.syn."0.15.13".quote}".default or false); }
      { "0.6.8".proc-macro =
        (f.quote."0.6.8".proc-macro or false) ||
        (syn."0.15.13"."proc-macro" or false) ||
        (f."syn"."0.15.13"."proc-macro" or false); }
    ];
    syn = fold recursiveUpdate {} [
      { "0.15.13".clone-impls =
        (f.syn."0.15.13".clone-impls or false) ||
        (f.syn."0.15.13".default or false) ||
        (syn."0.15.13"."default" or false); }
      { "0.15.13".default = (f.syn."0.15.13".default or true); }
      { "0.15.13".derive =
        (f.syn."0.15.13".derive or false) ||
        (f.syn."0.15.13".default or false) ||
        (syn."0.15.13"."default" or false); }
      { "0.15.13".parsing =
        (f.syn."0.15.13".parsing or false) ||
        (f.syn."0.15.13".default or false) ||
        (syn."0.15.13"."default" or false); }
      { "0.15.13".printing =
        (f.syn."0.15.13".printing or false) ||
        (f.syn."0.15.13".default or false) ||
        (syn."0.15.13"."default" or false); }
      { "0.15.13".proc-macro =
        (f.syn."0.15.13".proc-macro or false) ||
        (f.syn."0.15.13".default or false) ||
        (syn."0.15.13"."default" or false); }
      { "0.15.13".quote =
        (f.syn."0.15.13".quote or false) ||
        (f.syn."0.15.13".printing or false) ||
        (syn."0.15.13"."printing" or false); }
    ];
    unicode_xid."${deps.syn."0.15.13".unicode_xid}".default = true;
  }) [
    (features_.proc_macro2."${deps."syn"."0.15.13"."proc_macro2"}" deps)
    (features_.quote."${deps."syn"."0.15.13"."quote"}" deps)
    (features_.unicode_xid."${deps."syn"."0.15.13"."unicode_xid"}" deps)
  ];


  crates.take."0.1.0" = deps: { features?(features_.take."0.1.0" deps {}) }: buildRustCrate {
    crateName = "take";
    version = "0.1.0";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "17rfh39di5n8w9aghpic2r94cndi3dr04l60nkjylmxfxr3iwlhd";
  };
  features_.take."0.1.0" = deps: f: updateFeatures f (rec {
    take."0.1.0".default = (f.take."0.1.0".default or true);
  }) [];


  crates.termcolor."1.0.4" = deps: { features?(features_.termcolor."1.0.4" deps {}) }: buildRustCrate {
    crateName = "termcolor";
    version = "1.0.4";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "0xydrjc0bxg08llcbcmkka29szdrfklk4vh6l6mdd67ajifqw1mv";
    dependencies = (if kernel == "windows" then mapFeatures features ([
      (crates."wincolor"."${deps."termcolor"."1.0.4"."wincolor"}" deps)
    ]) else []);
  };
  features_.termcolor."1.0.4" = deps: f: updateFeatures f (rec {
    termcolor."1.0.4".default = (f.termcolor."1.0.4".default or true);
    wincolor."${deps.termcolor."1.0.4".wincolor}".default = true;
  }) [
    (features_.wincolor."${deps."termcolor"."1.0.4"."wincolor"}" deps)
  ];


  crates.termion."1.5.1" = deps: { features?(features_.termion."1.5.1" deps {}) }: buildRustCrate {
    crateName = "termion";
    version = "1.5.1";
    authors = [ "ticki <Ticki@users.noreply.github.com>" "gycos <alexandre.bury@gmail.com>" "IGI-111 <igi-111@protonmail.com>" ];
    sha256 = "02gq4vd8iws1f3gjrgrgpajsk2bk43nds5acbbb4s8dvrdvr8nf1";
    dependencies = (if !(kernel == "redox") then mapFeatures features ([
      (crates."libc"."${deps."termion"."1.5.1"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "redox" then mapFeatures features ([
      (crates."redox_syscall"."${deps."termion"."1.5.1"."redox_syscall"}" deps)
      (crates."redox_termios"."${deps."termion"."1.5.1"."redox_termios"}" deps)
    ]) else []);
  };
  features_.termion."1.5.1" = deps: f: updateFeatures f (rec {
    libc."${deps.termion."1.5.1".libc}".default = true;
    redox_syscall."${deps.termion."1.5.1".redox_syscall}".default = true;
    redox_termios."${deps.termion."1.5.1".redox_termios}".default = true;
    termion."1.5.1".default = (f.termion."1.5.1".default or true);
  }) [
    (features_.libc."${deps."termion"."1.5.1"."libc"}" deps)
    (features_.redox_syscall."${deps."termion"."1.5.1"."redox_syscall"}" deps)
    (features_.redox_termios."${deps."termion"."1.5.1"."redox_termios"}" deps)
  ];


  crates.thread_local."0.3.6" = deps: { features?(features_.thread_local."0.3.6" deps {}) }: buildRustCrate {
    crateName = "thread_local";
    version = "0.3.6";
    authors = [ "Amanieu d'Antras <amanieu@gmail.com>" ];
    sha256 = "02rksdwjmz2pw9bmgbb4c0bgkbq5z6nvg510sq1s6y2j1gam0c7i";
    dependencies = mapFeatures features ([
      (crates."lazy_static"."${deps."thread_local"."0.3.6"."lazy_static"}" deps)
    ]);
  };
  features_.thread_local."0.3.6" = deps: f: updateFeatures f (rec {
    lazy_static."${deps.thread_local."0.3.6".lazy_static}".default = true;
    thread_local."0.3.6".default = (f.thread_local."0.3.6".default or true);
  }) [
    (features_.lazy_static."${deps."thread_local"."0.3.6"."lazy_static"}" deps)
  ];


  crates.time."0.1.40" = deps: { features?(features_.time."0.1.40" deps {}) }: buildRustCrate {
    crateName = "time";
    version = "0.1.40";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0wgnbjamljz6bqxsd5axc4p2mmhkqfrryj4gf2yswjaxiw5dd01m";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."time"."0.1.40"."libc"}" deps)
    ])
      ++ (if kernel == "redox" then mapFeatures features ([
      (crates."redox_syscall"."${deps."time"."0.1.40"."redox_syscall"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."time"."0.1.40"."winapi"}" deps)
    ]) else []);
  };
  features_.time."0.1.40" = deps: f: updateFeatures f (rec {
    libc."${deps.time."0.1.40".libc}".default = true;
    redox_syscall."${deps.time."0.1.40".redox_syscall}".default = true;
    time."0.1.40".default = (f.time."0.1.40".default or true);
    winapi = fold recursiveUpdate {} [
      { "${deps.time."0.1.40".winapi}"."minwinbase" = true; }
      { "${deps.time."0.1.40".winapi}"."minwindef" = true; }
      { "${deps.time."0.1.40".winapi}"."ntdef" = true; }
      { "${deps.time."0.1.40".winapi}"."profileapi" = true; }
      { "${deps.time."0.1.40".winapi}"."std" = true; }
      { "${deps.time."0.1.40".winapi}"."sysinfoapi" = true; }
      { "${deps.time."0.1.40".winapi}"."timezoneapi" = true; }
      { "${deps.time."0.1.40".winapi}".default = true; }
    ];
  }) [
    (features_.libc."${deps."time"."0.1.40"."libc"}" deps)
    (features_.redox_syscall."${deps."time"."0.1.40"."redox_syscall"}" deps)
    (features_.winapi."${deps."time"."0.1.40"."winapi"}" deps)
  ];


  crates.tokio."0.1.11" = deps: { features?(features_.tokio."0.1.11" deps {}) }: buildRustCrate {
    crateName = "tokio";
    version = "0.1.11";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "1qzmy07idzv5klgv7yv92q0a528y8z2l97zm7daj2vxzbqp57amx";
    dependencies = mapFeatures features ([
      (crates."bytes"."${deps."tokio"."0.1.11"."bytes"}" deps)
      (crates."futures"."${deps."tokio"."0.1.11"."futures"}" deps)
      (crates."mio"."${deps."tokio"."0.1.11"."mio"}" deps)
      (crates."tokio_codec"."${deps."tokio"."0.1.11"."tokio_codec"}" deps)
      (crates."tokio_current_thread"."${deps."tokio"."0.1.11"."tokio_current_thread"}" deps)
      (crates."tokio_executor"."${deps."tokio"."0.1.11"."tokio_executor"}" deps)
      (crates."tokio_fs"."${deps."tokio"."0.1.11"."tokio_fs"}" deps)
      (crates."tokio_io"."${deps."tokio"."0.1.11"."tokio_io"}" deps)
      (crates."tokio_reactor"."${deps."tokio"."0.1.11"."tokio_reactor"}" deps)
      (crates."tokio_tcp"."${deps."tokio"."0.1.11"."tokio_tcp"}" deps)
      (crates."tokio_threadpool"."${deps."tokio"."0.1.11"."tokio_threadpool"}" deps)
      (crates."tokio_timer"."${deps."tokio"."0.1.11"."tokio_timer"}" deps)
      (crates."tokio_udp"."${deps."tokio"."0.1.11"."tokio_udp"}" deps)
    ])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."tokio_uds"."${deps."tokio"."0.1.11"."tokio_uds"}" deps)
    ]) else []);
    features = mkFeatures (features."tokio"."0.1.11" or {});
  };
  features_.tokio."0.1.11" = deps: f: updateFeatures f (rec {
    bytes."${deps.tokio."0.1.11".bytes}".default = true;
    futures."${deps.tokio."0.1.11".futures}".default = true;
    mio."${deps.tokio."0.1.11".mio}".default = true;
    tokio."0.1.11".default = (f.tokio."0.1.11".default or true);
    tokio_codec."${deps.tokio."0.1.11".tokio_codec}".default = true;
    tokio_current_thread."${deps.tokio."0.1.11".tokio_current_thread}".default = true;
    tokio_executor."${deps.tokio."0.1.11".tokio_executor}".default = true;
    tokio_fs."${deps.tokio."0.1.11".tokio_fs}".default = true;
    tokio_io."${deps.tokio."0.1.11".tokio_io}".default = true;
    tokio_reactor."${deps.tokio."0.1.11".tokio_reactor}".default = true;
    tokio_tcp."${deps.tokio."0.1.11".tokio_tcp}".default = true;
    tokio_threadpool."${deps.tokio."0.1.11".tokio_threadpool}".default = true;
    tokio_timer."${deps.tokio."0.1.11".tokio_timer}".default = true;
    tokio_udp."${deps.tokio."0.1.11".tokio_udp}".default = true;
    tokio_uds."${deps.tokio."0.1.11".tokio_uds}".default = true;
  }) [
    (features_.bytes."${deps."tokio"."0.1.11"."bytes"}" deps)
    (features_.futures."${deps."tokio"."0.1.11"."futures"}" deps)
    (features_.mio."${deps."tokio"."0.1.11"."mio"}" deps)
    (features_.tokio_codec."${deps."tokio"."0.1.11"."tokio_codec"}" deps)
    (features_.tokio_current_thread."${deps."tokio"."0.1.11"."tokio_current_thread"}" deps)
    (features_.tokio_executor."${deps."tokio"."0.1.11"."tokio_executor"}" deps)
    (features_.tokio_fs."${deps."tokio"."0.1.11"."tokio_fs"}" deps)
    (features_.tokio_io."${deps."tokio"."0.1.11"."tokio_io"}" deps)
    (features_.tokio_reactor."${deps."tokio"."0.1.11"."tokio_reactor"}" deps)
    (features_.tokio_tcp."${deps."tokio"."0.1.11"."tokio_tcp"}" deps)
    (features_.tokio_threadpool."${deps."tokio"."0.1.11"."tokio_threadpool"}" deps)
    (features_.tokio_timer."${deps."tokio"."0.1.11"."tokio_timer"}" deps)
    (features_.tokio_udp."${deps."tokio"."0.1.11"."tokio_udp"}" deps)
    (features_.tokio_uds."${deps."tokio"."0.1.11"."tokio_uds"}" deps)
  ];


  crates.tokio_codec."0.1.1" = deps: { features?(features_.tokio_codec."0.1.1" deps {}) }: buildRustCrate {
    crateName = "tokio-codec";
    version = "0.1.1";
    authors = [ "Carl Lerche <me@carllerche.com>" "Bryan Burgers <bryan@burgers.io>" ];
    sha256 = "0jc9lik540zyj4chbygg1rjh37m3zax8pd4bwcrwjmi1v56qwi4h";
    dependencies = mapFeatures features ([
      (crates."bytes"."${deps."tokio_codec"."0.1.1"."bytes"}" deps)
      (crates."futures"."${deps."tokio_codec"."0.1.1"."futures"}" deps)
      (crates."tokio_io"."${deps."tokio_codec"."0.1.1"."tokio_io"}" deps)
    ]);
  };
  features_.tokio_codec."0.1.1" = deps: f: updateFeatures f (rec {
    bytes."${deps.tokio_codec."0.1.1".bytes}".default = true;
    futures."${deps.tokio_codec."0.1.1".futures}".default = true;
    tokio_codec."0.1.1".default = (f.tokio_codec."0.1.1".default or true);
    tokio_io."${deps.tokio_codec."0.1.1".tokio_io}".default = true;
  }) [
    (features_.bytes."${deps."tokio_codec"."0.1.1"."bytes"}" deps)
    (features_.futures."${deps."tokio_codec"."0.1.1"."futures"}" deps)
    (features_.tokio_io."${deps."tokio_codec"."0.1.1"."tokio_io"}" deps)
  ];


  crates.tokio_core."0.1.17" = deps: { features?(features_.tokio_core."0.1.17" deps {}) }: buildRustCrate {
    crateName = "tokio-core";
    version = "0.1.17";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "1j6c5q3aakvb1hjx4r95xwl5ms8rp19k4qsr6v6ngwbvr6f9z6rs";
    dependencies = mapFeatures features ([
      (crates."bytes"."${deps."tokio_core"."0.1.17"."bytes"}" deps)
      (crates."futures"."${deps."tokio_core"."0.1.17"."futures"}" deps)
      (crates."iovec"."${deps."tokio_core"."0.1.17"."iovec"}" deps)
      (crates."log"."${deps."tokio_core"."0.1.17"."log"}" deps)
      (crates."mio"."${deps."tokio_core"."0.1.17"."mio"}" deps)
      (crates."scoped_tls"."${deps."tokio_core"."0.1.17"."scoped_tls"}" deps)
      (crates."tokio"."${deps."tokio_core"."0.1.17"."tokio"}" deps)
      (crates."tokio_executor"."${deps."tokio_core"."0.1.17"."tokio_executor"}" deps)
      (crates."tokio_io"."${deps."tokio_core"."0.1.17"."tokio_io"}" deps)
      (crates."tokio_reactor"."${deps."tokio_core"."0.1.17"."tokio_reactor"}" deps)
      (crates."tokio_timer"."${deps."tokio_core"."0.1.17"."tokio_timer"}" deps)
    ]);
  };
  features_.tokio_core."0.1.17" = deps: f: updateFeatures f (rec {
    bytes."${deps.tokio_core."0.1.17".bytes}".default = true;
    futures."${deps.tokio_core."0.1.17".futures}".default = true;
    iovec."${deps.tokio_core."0.1.17".iovec}".default = true;
    log."${deps.tokio_core."0.1.17".log}".default = true;
    mio."${deps.tokio_core."0.1.17".mio}".default = true;
    scoped_tls."${deps.tokio_core."0.1.17".scoped_tls}".default = true;
    tokio."${deps.tokio_core."0.1.17".tokio}".default = true;
    tokio_core."0.1.17".default = (f.tokio_core."0.1.17".default or true);
    tokio_executor."${deps.tokio_core."0.1.17".tokio_executor}".default = true;
    tokio_io."${deps.tokio_core."0.1.17".tokio_io}".default = true;
    tokio_reactor."${deps.tokio_core."0.1.17".tokio_reactor}".default = true;
    tokio_timer."${deps.tokio_core."0.1.17".tokio_timer}".default = true;
  }) [
    (features_.bytes."${deps."tokio_core"."0.1.17"."bytes"}" deps)
    (features_.futures."${deps."tokio_core"."0.1.17"."futures"}" deps)
    (features_.iovec."${deps."tokio_core"."0.1.17"."iovec"}" deps)
    (features_.log."${deps."tokio_core"."0.1.17"."log"}" deps)
    (features_.mio."${deps."tokio_core"."0.1.17"."mio"}" deps)
    (features_.scoped_tls."${deps."tokio_core"."0.1.17"."scoped_tls"}" deps)
    (features_.tokio."${deps."tokio_core"."0.1.17"."tokio"}" deps)
    (features_.tokio_executor."${deps."tokio_core"."0.1.17"."tokio_executor"}" deps)
    (features_.tokio_io."${deps."tokio_core"."0.1.17"."tokio_io"}" deps)
    (features_.tokio_reactor."${deps."tokio_core"."0.1.17"."tokio_reactor"}" deps)
    (features_.tokio_timer."${deps."tokio_core"."0.1.17"."tokio_timer"}" deps)
  ];


  crates.tokio_current_thread."0.1.3" = deps: { features?(features_.tokio_current_thread."0.1.3" deps {}) }: buildRustCrate {
    crateName = "tokio-current-thread";
    version = "0.1.3";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0nay6ar70s5xqx1hjqcpd1k3dp2n5r5smlkgjbfyyw5sjkfzf3kz";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."tokio_current_thread"."0.1.3"."futures"}" deps)
      (crates."tokio_executor"."${deps."tokio_current_thread"."0.1.3"."tokio_executor"}" deps)
    ]);
  };
  features_.tokio_current_thread."0.1.3" = deps: f: updateFeatures f (rec {
    futures."${deps.tokio_current_thread."0.1.3".futures}".default = true;
    tokio_current_thread."0.1.3".default = (f.tokio_current_thread."0.1.3".default or true);
    tokio_executor."${deps.tokio_current_thread."0.1.3".tokio_executor}".default = true;
  }) [
    (features_.futures."${deps."tokio_current_thread"."0.1.3"."futures"}" deps)
    (features_.tokio_executor."${deps."tokio_current_thread"."0.1.3"."tokio_executor"}" deps)
  ];


  crates.tokio_executor."0.1.5" = deps: { features?(features_.tokio_executor."0.1.5" deps {}) }: buildRustCrate {
    crateName = "tokio-executor";
    version = "0.1.5";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "15j2ybs8w38gncgbxkvp2qsp6wl62ibi3rns0vlwggx7svmx4bf3";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."tokio_executor"."0.1.5"."futures"}" deps)
    ]);
  };
  features_.tokio_executor."0.1.5" = deps: f: updateFeatures f (rec {
    futures."${deps.tokio_executor."0.1.5".futures}".default = true;
    tokio_executor."0.1.5".default = (f.tokio_executor."0.1.5".default or true);
  }) [
    (features_.futures."${deps."tokio_executor"."0.1.5"."futures"}" deps)
  ];


  crates.tokio_fs."0.1.4" = deps: { features?(features_.tokio_fs."0.1.4" deps {}) }: buildRustCrate {
    crateName = "tokio-fs";
    version = "0.1.4";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "05bpc1p1apb4jfw18i84agwwar57zn07d7smqvslpzagd9b3sd31";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."tokio_fs"."0.1.4"."futures"}" deps)
      (crates."tokio_io"."${deps."tokio_fs"."0.1.4"."tokio_io"}" deps)
      (crates."tokio_threadpool"."${deps."tokio_fs"."0.1.4"."tokio_threadpool"}" deps)
    ]);
  };
  features_.tokio_fs."0.1.4" = deps: f: updateFeatures f (rec {
    futures."${deps.tokio_fs."0.1.4".futures}".default = true;
    tokio_fs."0.1.4".default = (f.tokio_fs."0.1.4".default or true);
    tokio_io."${deps.tokio_fs."0.1.4".tokio_io}".default = true;
    tokio_threadpool."${deps.tokio_fs."0.1.4".tokio_threadpool}".default = true;
  }) [
    (features_.futures."${deps."tokio_fs"."0.1.4"."futures"}" deps)
    (features_.tokio_io."${deps."tokio_fs"."0.1.4"."tokio_io"}" deps)
    (features_.tokio_threadpool."${deps."tokio_fs"."0.1.4"."tokio_threadpool"}" deps)
  ];


  crates.tokio_io."0.1.10" = deps: { features?(features_.tokio_io."0.1.10" deps {}) }: buildRustCrate {
    crateName = "tokio-io";
    version = "0.1.10";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "14d65rqa5rb2msgkz2xn40cavs4m7f4qyi7vnfv98v7f10l9wlay";
    dependencies = mapFeatures features ([
      (crates."bytes"."${deps."tokio_io"."0.1.10"."bytes"}" deps)
      (crates."futures"."${deps."tokio_io"."0.1.10"."futures"}" deps)
      (crates."log"."${deps."tokio_io"."0.1.10"."log"}" deps)
    ]);
  };
  features_.tokio_io."0.1.10" = deps: f: updateFeatures f (rec {
    bytes."${deps.tokio_io."0.1.10".bytes}".default = true;
    futures."${deps.tokio_io."0.1.10".futures}".default = true;
    log."${deps.tokio_io."0.1.10".log}".default = true;
    tokio_io."0.1.10".default = (f.tokio_io."0.1.10".default or true);
  }) [
    (features_.bytes."${deps."tokio_io"."0.1.10"."bytes"}" deps)
    (features_.futures."${deps."tokio_io"."0.1.10"."futures"}" deps)
    (features_.log."${deps."tokio_io"."0.1.10"."log"}" deps)
  ];


  crates.tokio_proto."0.1.1" = deps: { features?(features_.tokio_proto."0.1.1" deps {}) }: buildRustCrate {
    crateName = "tokio-proto";
    version = "0.1.1";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "030q9h8pn1ngm80klff5irglxxki60hf5maw0mppmmr46k773z66";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."tokio_proto"."0.1.1"."futures"}" deps)
      (crates."log"."${deps."tokio_proto"."0.1.1"."log"}" deps)
      (crates."net2"."${deps."tokio_proto"."0.1.1"."net2"}" deps)
      (crates."rand"."${deps."tokio_proto"."0.1.1"."rand"}" deps)
      (crates."slab"."${deps."tokio_proto"."0.1.1"."slab"}" deps)
      (crates."smallvec"."${deps."tokio_proto"."0.1.1"."smallvec"}" deps)
      (crates."take"."${deps."tokio_proto"."0.1.1"."take"}" deps)
      (crates."tokio_core"."${deps."tokio_proto"."0.1.1"."tokio_core"}" deps)
      (crates."tokio_io"."${deps."tokio_proto"."0.1.1"."tokio_io"}" deps)
      (crates."tokio_service"."${deps."tokio_proto"."0.1.1"."tokio_service"}" deps)
    ]);
  };
  features_.tokio_proto."0.1.1" = deps: f: updateFeatures f (rec {
    futures."${deps.tokio_proto."0.1.1".futures}".default = true;
    log."${deps.tokio_proto."0.1.1".log}".default = true;
    net2."${deps.tokio_proto."0.1.1".net2}".default = true;
    rand."${deps.tokio_proto."0.1.1".rand}".default = true;
    slab."${deps.tokio_proto."0.1.1".slab}".default = true;
    smallvec."${deps.tokio_proto."0.1.1".smallvec}".default = true;
    take."${deps.tokio_proto."0.1.1".take}".default = true;
    tokio_core."${deps.tokio_proto."0.1.1".tokio_core}".default = true;
    tokio_io."${deps.tokio_proto."0.1.1".tokio_io}".default = true;
    tokio_proto."0.1.1".default = (f.tokio_proto."0.1.1".default or true);
    tokio_service."${deps.tokio_proto."0.1.1".tokio_service}".default = true;
  }) [
    (features_.futures."${deps."tokio_proto"."0.1.1"."futures"}" deps)
    (features_.log."${deps."tokio_proto"."0.1.1"."log"}" deps)
    (features_.net2."${deps."tokio_proto"."0.1.1"."net2"}" deps)
    (features_.rand."${deps."tokio_proto"."0.1.1"."rand"}" deps)
    (features_.slab."${deps."tokio_proto"."0.1.1"."slab"}" deps)
    (features_.smallvec."${deps."tokio_proto"."0.1.1"."smallvec"}" deps)
    (features_.take."${deps."tokio_proto"."0.1.1"."take"}" deps)
    (features_.tokio_core."${deps."tokio_proto"."0.1.1"."tokio_core"}" deps)
    (features_.tokio_io."${deps."tokio_proto"."0.1.1"."tokio_io"}" deps)
    (features_.tokio_service."${deps."tokio_proto"."0.1.1"."tokio_service"}" deps)
  ];


  crates.tokio_reactor."0.1.6" = deps: { features?(features_.tokio_reactor."0.1.6" deps {}) }: buildRustCrate {
    crateName = "tokio-reactor";
    version = "0.1.6";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0xjy2b4pfyiyhb8whm0b1xxa3n6v5w8hl0p0cjqqrasci6v53n3s";
    dependencies = mapFeatures features ([
      (crates."crossbeam_utils"."${deps."tokio_reactor"."0.1.6"."crossbeam_utils"}" deps)
      (crates."futures"."${deps."tokio_reactor"."0.1.6"."futures"}" deps)
      (crates."lazy_static"."${deps."tokio_reactor"."0.1.6"."lazy_static"}" deps)
      (crates."log"."${deps."tokio_reactor"."0.1.6"."log"}" deps)
      (crates."mio"."${deps."tokio_reactor"."0.1.6"."mio"}" deps)
      (crates."num_cpus"."${deps."tokio_reactor"."0.1.6"."num_cpus"}" deps)
      (crates."parking_lot"."${deps."tokio_reactor"."0.1.6"."parking_lot"}" deps)
      (crates."slab"."${deps."tokio_reactor"."0.1.6"."slab"}" deps)
      (crates."tokio_executor"."${deps."tokio_reactor"."0.1.6"."tokio_executor"}" deps)
      (crates."tokio_io"."${deps."tokio_reactor"."0.1.6"."tokio_io"}" deps)
    ]);
  };
  features_.tokio_reactor."0.1.6" = deps: f: updateFeatures f (rec {
    crossbeam_utils."${deps.tokio_reactor."0.1.6".crossbeam_utils}".default = true;
    futures."${deps.tokio_reactor."0.1.6".futures}".default = true;
    lazy_static."${deps.tokio_reactor."0.1.6".lazy_static}".default = true;
    log."${deps.tokio_reactor."0.1.6".log}".default = true;
    mio."${deps.tokio_reactor."0.1.6".mio}".default = true;
    num_cpus."${deps.tokio_reactor."0.1.6".num_cpus}".default = true;
    parking_lot."${deps.tokio_reactor."0.1.6".parking_lot}".default = true;
    slab."${deps.tokio_reactor."0.1.6".slab}".default = true;
    tokio_executor."${deps.tokio_reactor."0.1.6".tokio_executor}".default = true;
    tokio_io."${deps.tokio_reactor."0.1.6".tokio_io}".default = true;
    tokio_reactor."0.1.6".default = (f.tokio_reactor."0.1.6".default or true);
  }) [
    (features_.crossbeam_utils."${deps."tokio_reactor"."0.1.6"."crossbeam_utils"}" deps)
    (features_.futures."${deps."tokio_reactor"."0.1.6"."futures"}" deps)
    (features_.lazy_static."${deps."tokio_reactor"."0.1.6"."lazy_static"}" deps)
    (features_.log."${deps."tokio_reactor"."0.1.6"."log"}" deps)
    (features_.mio."${deps."tokio_reactor"."0.1.6"."mio"}" deps)
    (features_.num_cpus."${deps."tokio_reactor"."0.1.6"."num_cpus"}" deps)
    (features_.parking_lot."${deps."tokio_reactor"."0.1.6"."parking_lot"}" deps)
    (features_.slab."${deps."tokio_reactor"."0.1.6"."slab"}" deps)
    (features_.tokio_executor."${deps."tokio_reactor"."0.1.6"."tokio_executor"}" deps)
    (features_.tokio_io."${deps."tokio_reactor"."0.1.6"."tokio_io"}" deps)
  ];


  crates.tokio_service."0.1.0" = deps: { features?(features_.tokio_service."0.1.0" deps {}) }: buildRustCrate {
    crateName = "tokio-service";
    version = "0.1.0";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0c85wm5qz9fabg0k6k763j89m43n6max72d3a8sxcs940id6qmih";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."tokio_service"."0.1.0"."futures"}" deps)
    ]);
  };
  features_.tokio_service."0.1.0" = deps: f: updateFeatures f (rec {
    futures."${deps.tokio_service."0.1.0".futures}".default = true;
    tokio_service."0.1.0".default = (f.tokio_service."0.1.0".default or true);
  }) [
    (features_.futures."${deps."tokio_service"."0.1.0"."futures"}" deps)
  ];


  crates.tokio_tcp."0.1.2" = deps: { features?(features_.tokio_tcp."0.1.2" deps {}) }: buildRustCrate {
    crateName = "tokio-tcp";
    version = "0.1.2";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0yvfwybqnyca24aj9as8rgydamjq0wrd9xbxxkjcasvsdmsv6z1d";
    dependencies = mapFeatures features ([
      (crates."bytes"."${deps."tokio_tcp"."0.1.2"."bytes"}" deps)
      (crates."futures"."${deps."tokio_tcp"."0.1.2"."futures"}" deps)
      (crates."iovec"."${deps."tokio_tcp"."0.1.2"."iovec"}" deps)
      (crates."mio"."${deps."tokio_tcp"."0.1.2"."mio"}" deps)
      (crates."tokio_io"."${deps."tokio_tcp"."0.1.2"."tokio_io"}" deps)
      (crates."tokio_reactor"."${deps."tokio_tcp"."0.1.2"."tokio_reactor"}" deps)
    ]);
  };
  features_.tokio_tcp."0.1.2" = deps: f: updateFeatures f (rec {
    bytes."${deps.tokio_tcp."0.1.2".bytes}".default = true;
    futures."${deps.tokio_tcp."0.1.2".futures}".default = true;
    iovec."${deps.tokio_tcp."0.1.2".iovec}".default = true;
    mio."${deps.tokio_tcp."0.1.2".mio}".default = true;
    tokio_io."${deps.tokio_tcp."0.1.2".tokio_io}".default = true;
    tokio_reactor."${deps.tokio_tcp."0.1.2".tokio_reactor}".default = true;
    tokio_tcp."0.1.2".default = (f.tokio_tcp."0.1.2".default or true);
  }) [
    (features_.bytes."${deps."tokio_tcp"."0.1.2"."bytes"}" deps)
    (features_.futures."${deps."tokio_tcp"."0.1.2"."futures"}" deps)
    (features_.iovec."${deps."tokio_tcp"."0.1.2"."iovec"}" deps)
    (features_.mio."${deps."tokio_tcp"."0.1.2"."mio"}" deps)
    (features_.tokio_io."${deps."tokio_tcp"."0.1.2"."tokio_io"}" deps)
    (features_.tokio_reactor."${deps."tokio_tcp"."0.1.2"."tokio_reactor"}" deps)
  ];


  crates.tokio_threadpool."0.1.8" = deps: { features?(features_.tokio_threadpool."0.1.8" deps {}) }: buildRustCrate {
    crateName = "tokio-threadpool";
    version = "0.1.8";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0dgnpgg3i8rbx66nb9crdlyn3dznzs0kb73sdq652xc16c9vhkc9";
    dependencies = mapFeatures features ([
      (crates."crossbeam_deque"."${deps."tokio_threadpool"."0.1.8"."crossbeam_deque"}" deps)
      (crates."crossbeam_utils"."${deps."tokio_threadpool"."0.1.8"."crossbeam_utils"}" deps)
      (crates."futures"."${deps."tokio_threadpool"."0.1.8"."futures"}" deps)
      (crates."log"."${deps."tokio_threadpool"."0.1.8"."log"}" deps)
      (crates."num_cpus"."${deps."tokio_threadpool"."0.1.8"."num_cpus"}" deps)
      (crates."rand"."${deps."tokio_threadpool"."0.1.8"."rand"}" deps)
      (crates."tokio_executor"."${deps."tokio_threadpool"."0.1.8"."tokio_executor"}" deps)
    ]);
  };
  features_.tokio_threadpool."0.1.8" = deps: f: updateFeatures f (rec {
    crossbeam_deque."${deps.tokio_threadpool."0.1.8".crossbeam_deque}".default = true;
    crossbeam_utils."${deps.tokio_threadpool."0.1.8".crossbeam_utils}".default = true;
    futures."${deps.tokio_threadpool."0.1.8".futures}".default = true;
    log."${deps.tokio_threadpool."0.1.8".log}".default = true;
    num_cpus."${deps.tokio_threadpool."0.1.8".num_cpus}".default = true;
    rand."${deps.tokio_threadpool."0.1.8".rand}".default = true;
    tokio_executor."${deps.tokio_threadpool."0.1.8".tokio_executor}".default = true;
    tokio_threadpool."0.1.8".default = (f.tokio_threadpool."0.1.8".default or true);
  }) [
    (features_.crossbeam_deque."${deps."tokio_threadpool"."0.1.8"."crossbeam_deque"}" deps)
    (features_.crossbeam_utils."${deps."tokio_threadpool"."0.1.8"."crossbeam_utils"}" deps)
    (features_.futures."${deps."tokio_threadpool"."0.1.8"."futures"}" deps)
    (features_.log."${deps."tokio_threadpool"."0.1.8"."log"}" deps)
    (features_.num_cpus."${deps."tokio_threadpool"."0.1.8"."num_cpus"}" deps)
    (features_.rand."${deps."tokio_threadpool"."0.1.8"."rand"}" deps)
    (features_.tokio_executor."${deps."tokio_threadpool"."0.1.8"."tokio_executor"}" deps)
  ];


  crates.tokio_timer."0.2.7" = deps: { features?(features_.tokio_timer."0.2.7" deps {}) }: buildRustCrate {
    crateName = "tokio-timer";
    version = "0.2.7";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0xajkivlrmg7ygc46jh24bv35vpw51kfm95b6bhnpifl5g04cc1f";
    dependencies = mapFeatures features ([
      (crates."crossbeam_utils"."${deps."tokio_timer"."0.2.7"."crossbeam_utils"}" deps)
      (crates."futures"."${deps."tokio_timer"."0.2.7"."futures"}" deps)
      (crates."slab"."${deps."tokio_timer"."0.2.7"."slab"}" deps)
      (crates."tokio_executor"."${deps."tokio_timer"."0.2.7"."tokio_executor"}" deps)
    ]);
  };
  features_.tokio_timer."0.2.7" = deps: f: updateFeatures f (rec {
    crossbeam_utils."${deps.tokio_timer."0.2.7".crossbeam_utils}".default = true;
    futures."${deps.tokio_timer."0.2.7".futures}".default = true;
    slab."${deps.tokio_timer."0.2.7".slab}".default = true;
    tokio_executor."${deps.tokio_timer."0.2.7".tokio_executor}".default = true;
    tokio_timer."0.2.7".default = (f.tokio_timer."0.2.7".default or true);
  }) [
    (features_.crossbeam_utils."${deps."tokio_timer"."0.2.7"."crossbeam_utils"}" deps)
    (features_.futures."${deps."tokio_timer"."0.2.7"."futures"}" deps)
    (features_.slab."${deps."tokio_timer"."0.2.7"."slab"}" deps)
    (features_.tokio_executor."${deps."tokio_timer"."0.2.7"."tokio_executor"}" deps)
  ];


  crates.tokio_udp."0.1.2" = deps: { features?(features_.tokio_udp."0.1.2" deps {}) }: buildRustCrate {
    crateName = "tokio-udp";
    version = "0.1.2";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0wl30g8i66pylg1xgaqa5vwjg4hw7m714r6qx7jz0a42dc11ij48";
    dependencies = mapFeatures features ([
      (crates."bytes"."${deps."tokio_udp"."0.1.2"."bytes"}" deps)
      (crates."futures"."${deps."tokio_udp"."0.1.2"."futures"}" deps)
      (crates."log"."${deps."tokio_udp"."0.1.2"."log"}" deps)
      (crates."mio"."${deps."tokio_udp"."0.1.2"."mio"}" deps)
      (crates."tokio_codec"."${deps."tokio_udp"."0.1.2"."tokio_codec"}" deps)
      (crates."tokio_io"."${deps."tokio_udp"."0.1.2"."tokio_io"}" deps)
      (crates."tokio_reactor"."${deps."tokio_udp"."0.1.2"."tokio_reactor"}" deps)
    ]);
  };
  features_.tokio_udp."0.1.2" = deps: f: updateFeatures f (rec {
    bytes."${deps.tokio_udp."0.1.2".bytes}".default = true;
    futures."${deps.tokio_udp."0.1.2".futures}".default = true;
    log."${deps.tokio_udp."0.1.2".log}".default = true;
    mio."${deps.tokio_udp."0.1.2".mio}".default = true;
    tokio_codec."${deps.tokio_udp."0.1.2".tokio_codec}".default = true;
    tokio_io."${deps.tokio_udp."0.1.2".tokio_io}".default = true;
    tokio_reactor."${deps.tokio_udp."0.1.2".tokio_reactor}".default = true;
    tokio_udp."0.1.2".default = (f.tokio_udp."0.1.2".default or true);
  }) [
    (features_.bytes."${deps."tokio_udp"."0.1.2"."bytes"}" deps)
    (features_.futures."${deps."tokio_udp"."0.1.2"."futures"}" deps)
    (features_.log."${deps."tokio_udp"."0.1.2"."log"}" deps)
    (features_.mio."${deps."tokio_udp"."0.1.2"."mio"}" deps)
    (features_.tokio_codec."${deps."tokio_udp"."0.1.2"."tokio_codec"}" deps)
    (features_.tokio_io."${deps."tokio_udp"."0.1.2"."tokio_io"}" deps)
    (features_.tokio_reactor."${deps."tokio_udp"."0.1.2"."tokio_reactor"}" deps)
  ];


  crates.tokio_uds."0.2.3" = deps: { features?(features_.tokio_uds."0.2.3" deps {}) }: buildRustCrate {
    crateName = "tokio-uds";
    version = "0.2.3";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "035xn79ppj52azh7j6ydjz73y6d86k3jcc5dbc9qz6xb36rdzj6q";
    dependencies = mapFeatures features ([
      (crates."bytes"."${deps."tokio_uds"."0.2.3"."bytes"}" deps)
      (crates."futures"."${deps."tokio_uds"."0.2.3"."futures"}" deps)
      (crates."iovec"."${deps."tokio_uds"."0.2.3"."iovec"}" deps)
      (crates."libc"."${deps."tokio_uds"."0.2.3"."libc"}" deps)
      (crates."log"."${deps."tokio_uds"."0.2.3"."log"}" deps)
      (crates."mio"."${deps."tokio_uds"."0.2.3"."mio"}" deps)
      (crates."mio_uds"."${deps."tokio_uds"."0.2.3"."mio_uds"}" deps)
      (crates."tokio_io"."${deps."tokio_uds"."0.2.3"."tokio_io"}" deps)
      (crates."tokio_reactor"."${deps."tokio_uds"."0.2.3"."tokio_reactor"}" deps)
    ]);
  };
  features_.tokio_uds."0.2.3" = deps: f: updateFeatures f (rec {
    bytes."${deps.tokio_uds."0.2.3".bytes}".default = true;
    futures."${deps.tokio_uds."0.2.3".futures}".default = true;
    iovec."${deps.tokio_uds."0.2.3".iovec}".default = true;
    libc."${deps.tokio_uds."0.2.3".libc}".default = true;
    log."${deps.tokio_uds."0.2.3".log}".default = true;
    mio."${deps.tokio_uds."0.2.3".mio}".default = true;
    mio_uds."${deps.tokio_uds."0.2.3".mio_uds}".default = true;
    tokio_io."${deps.tokio_uds."0.2.3".tokio_io}".default = true;
    tokio_reactor."${deps.tokio_uds."0.2.3".tokio_reactor}".default = true;
    tokio_uds."0.2.3".default = (f.tokio_uds."0.2.3".default or true);
  }) [
    (features_.bytes."${deps."tokio_uds"."0.2.3"."bytes"}" deps)
    (features_.futures."${deps."tokio_uds"."0.2.3"."futures"}" deps)
    (features_.iovec."${deps."tokio_uds"."0.2.3"."iovec"}" deps)
    (features_.libc."${deps."tokio_uds"."0.2.3"."libc"}" deps)
    (features_.log."${deps."tokio_uds"."0.2.3"."log"}" deps)
    (features_.mio."${deps."tokio_uds"."0.2.3"."mio"}" deps)
    (features_.mio_uds."${deps."tokio_uds"."0.2.3"."mio_uds"}" deps)
    (features_.tokio_io."${deps."tokio_uds"."0.2.3"."tokio_io"}" deps)
    (features_.tokio_reactor."${deps."tokio_uds"."0.2.3"."tokio_reactor"}" deps)
  ];


  crates.try_lock."0.1.0" = deps: { features?(features_.try_lock."0.1.0" deps {}) }: buildRustCrate {
    crateName = "try-lock";
    version = "0.1.0";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "0kfrqrb2xkjig54s3qfy80dpldknr19p3rmp0n82yk5929j879k3";
  };
  features_.try_lock."0.1.0" = deps: f: updateFeatures f (rec {
    try_lock."0.1.0".default = (f.try_lock."0.1.0".default or true);
  }) [];


  crates.ucd_util."0.1.1" = deps: { features?(features_.ucd_util."0.1.1" deps {}) }: buildRustCrate {
    crateName = "ucd-util";
    version = "0.1.1";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "02a8h3siipx52b832xc8m8rwasj6nx9jpiwfldw8hp6k205hgkn0";
  };
  features_.ucd_util."0.1.1" = deps: f: updateFeatures f (rec {
    ucd_util."0.1.1".default = (f.ucd_util."0.1.1".default or true);
  }) [];


  crates.unicase."2.2.0" = deps: { features?(features_.unicase."2.2.0" deps {}) }: buildRustCrate {
    crateName = "unicase";
    version = "2.2.0";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "0p8fj4rdjk9k15s552bl6vpidjcf4jzddzkz6vgagb2i84xlvfxc";
    build = "build.rs";

    buildDependencies = mapFeatures features ([
      (crates."version_check"."${deps."unicase"."2.2.0"."version_check"}" deps)
    ]);
    features = mkFeatures (features."unicase"."2.2.0" or {});
  };
  features_.unicase."2.2.0" = deps: f: updateFeatures f (rec {
    unicase."2.2.0".default = (f.unicase."2.2.0".default or true);
    version_check."${deps.unicase."2.2.0".version_check}".default = true;
  }) [
    (features_.version_check."${deps."unicase"."2.2.0"."version_check"}" deps)
  ];


  crates.unicode_normalization."0.1.7" = deps: { features?(features_.unicode_normalization."0.1.7" deps {}) }: buildRustCrate {
    crateName = "unicode-normalization";
    version = "0.1.7";
    authors = [ "kwantam <kwantam@gmail.com>" ];
    sha256 = "1da2hv800pd0wilmn4idwpgv5p510hjxizjcfv6xzb40xcsjd8gs";
  };
  features_.unicode_normalization."0.1.7" = deps: f: updateFeatures f (rec {
    unicode_normalization."0.1.7".default = (f.unicode_normalization."0.1.7".default or true);
  }) [];


  crates.unicode_xid."0.1.0" = deps: { features?(features_.unicode_xid."0.1.0" deps {}) }: buildRustCrate {
    crateName = "unicode-xid";
    version = "0.1.0";
    authors = [ "erick.tryzelaar <erick.tryzelaar@gmail.com>" "kwantam <kwantam@gmail.com>" ];
    sha256 = "05wdmwlfzxhq3nhsxn6wx4q8dhxzzfb9szsz6wiw092m1rjj01zj";
    features = mkFeatures (features."unicode_xid"."0.1.0" or {});
  };
  features_.unicode_xid."0.1.0" = deps: f: updateFeatures f (rec {
    unicode_xid."0.1.0".default = (f.unicode_xid."0.1.0".default or true);
  }) [];


  crates.unreachable."1.0.0" = deps: { features?(features_.unreachable."1.0.0" deps {}) }: buildRustCrate {
    crateName = "unreachable";
    version = "1.0.0";
    authors = [ "Jonathan Reem <jonathan.reem@gmail.com>" ];
    sha256 = "1am8czbk5wwr25gbp2zr007744fxjshhdqjz9liz7wl4pnv3whcf";
    dependencies = mapFeatures features ([
      (crates."void"."${deps."unreachable"."1.0.0"."void"}" deps)
    ]);
  };
  features_.unreachable."1.0.0" = deps: f: updateFeatures f (rec {
    unreachable."1.0.0".default = (f.unreachable."1.0.0".default or true);
    void."${deps.unreachable."1.0.0".void}".default = (f.void."${deps.unreachable."1.0.0".void}".default or false);
  }) [
    (features_.void."${deps."unreachable"."1.0.0"."void"}" deps)
  ];


  crates.utf8_ranges."1.0.1" = deps: { features?(features_.utf8_ranges."1.0.1" deps {}) }: buildRustCrate {
    crateName = "utf8-ranges";
    version = "1.0.1";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "1s56ihd2c8ba6191078wivvv59247szaiszrh8x2rxqfsxlfrnpp";
  };
  features_.utf8_ranges."1.0.1" = deps: f: updateFeatures f (rec {
    utf8_ranges."1.0.1".default = (f.utf8_ranges."1.0.1".default or true);
  }) [];


  crates.version_check."0.1.5" = deps: { features?(features_.version_check."0.1.5" deps {}) }: buildRustCrate {
    crateName = "version_check";
    version = "0.1.5";
    authors = [ "Sergio Benitez <sb@sergio.bz>" ];
    sha256 = "1yrx9xblmwbafw2firxyqbj8f771kkzfd24n3q7xgwiqyhi0y8qd";
  };
  features_.version_check."0.1.5" = deps: f: updateFeatures f (rec {
    version_check."0.1.5".default = (f.version_check."0.1.5".default or true);
  }) [];


  crates.void."1.0.2" = deps: { features?(features_.void."1.0.2" deps {}) }: buildRustCrate {
    crateName = "void";
    version = "1.0.2";
    authors = [ "Jonathan Reem <jonathan.reem@gmail.com>" ];
    sha256 = "0h1dm0dx8dhf56a83k68mijyxigqhizpskwxfdrs1drwv2cdclv3";
    features = mkFeatures (features."void"."1.0.2" or {});
  };
  features_.void."1.0.2" = deps: f: updateFeatures f (rec {
    void = fold recursiveUpdate {} [
      { "1.0.2".default = (f.void."1.0.2".default or true); }
      { "1.0.2".std =
        (f.void."1.0.2".std or false) ||
        (f.void."1.0.2".default or false) ||
        (void."1.0.2"."default" or false); }
    ];
  }) [];


  crates.want."0.0.4" = deps: { features?(features_.want."0.0.4" deps {}) }: buildRustCrate {
    crateName = "want";
    version = "0.0.4";
    authors = [ "Sean McArthur <sean@seanmonstar.com>" ];
    sha256 = "1l1qy4pvg5q71nrzfjldw9xzqhhgicj4slly1bal89hr2aaibpy0";
    dependencies = mapFeatures features ([
      (crates."futures"."${deps."want"."0.0.4"."futures"}" deps)
      (crates."log"."${deps."want"."0.0.4"."log"}" deps)
      (crates."try_lock"."${deps."want"."0.0.4"."try_lock"}" deps)
    ]);
  };
  features_.want."0.0.4" = deps: f: updateFeatures f (rec {
    futures."${deps.want."0.0.4".futures}".default = true;
    log."${deps.want."0.0.4".log}".default = true;
    try_lock."${deps.want."0.0.4".try_lock}".default = true;
    want."0.0.4".default = (f.want."0.0.4".default or true);
  }) [
    (features_.futures."${deps."want"."0.0.4"."futures"}" deps)
    (features_.log."${deps."want"."0.0.4"."log"}" deps)
    (features_.try_lock."${deps."want"."0.0.4"."try_lock"}" deps)
  ];


  crates.winapi."0.2.8" = deps: { features?(features_.winapi."0.2.8" deps {}) }: buildRustCrate {
    crateName = "winapi";
    version = "0.2.8";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "0a45b58ywf12vb7gvj6h3j264nydynmzyqz8d8rqxsj6icqv82as";
  };
  features_.winapi."0.2.8" = deps: f: updateFeatures f (rec {
    winapi."0.2.8".default = (f.winapi."0.2.8".default or true);
  }) [];


  crates.winapi."0.3.6" = deps: { features?(features_.winapi."0.3.6" deps {}) }: buildRustCrate {
    crateName = "winapi";
    version = "0.3.6";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1d9jfp4cjd82sr1q4dgdlrkvm33zhhav9d7ihr0nivqbncr059m4";
    build = "build.rs";
    dependencies = (if kernel == "i686-pc-windows-gnu" then mapFeatures features ([
      (crates."winapi_i686_pc_windows_gnu"."${deps."winapi"."0.3.6"."winapi_i686_pc_windows_gnu"}" deps)
    ]) else [])
      ++ (if kernel == "x86_64-pc-windows-gnu" then mapFeatures features ([
      (crates."winapi_x86_64_pc_windows_gnu"."${deps."winapi"."0.3.6"."winapi_x86_64_pc_windows_gnu"}" deps)
    ]) else []);
    features = mkFeatures (features."winapi"."0.3.6" or {});
  };
  features_.winapi."0.3.6" = deps: f: updateFeatures f (rec {
    winapi."0.3.6".default = (f.winapi."0.3.6".default or true);
    winapi_i686_pc_windows_gnu."${deps.winapi."0.3.6".winapi_i686_pc_windows_gnu}".default = true;
    winapi_x86_64_pc_windows_gnu."${deps.winapi."0.3.6".winapi_x86_64_pc_windows_gnu}".default = true;
  }) [
    (features_.winapi_i686_pc_windows_gnu."${deps."winapi"."0.3.6"."winapi_i686_pc_windows_gnu"}" deps)
    (features_.winapi_x86_64_pc_windows_gnu."${deps."winapi"."0.3.6"."winapi_x86_64_pc_windows_gnu"}" deps)
  ];


  crates.winapi_build."0.1.1" = deps: { features?(features_.winapi_build."0.1.1" deps {}) }: buildRustCrate {
    crateName = "winapi-build";
    version = "0.1.1";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1lxlpi87rkhxcwp2ykf1ldw3p108hwm24nywf3jfrvmff4rjhqga";
    libName = "build";
  };
  features_.winapi_build."0.1.1" = deps: f: updateFeatures f (rec {
    winapi_build."0.1.1".default = (f.winapi_build."0.1.1".default or true);
  }) [];


  crates.winapi_i686_pc_windows_gnu."0.4.0" = deps: { features?(features_.winapi_i686_pc_windows_gnu."0.4.0" deps {}) }: buildRustCrate {
    crateName = "winapi-i686-pc-windows-gnu";
    version = "0.4.0";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "05ihkij18r4gamjpxj4gra24514can762imjzlmak5wlzidplzrp";
    build = "build.rs";
  };
  features_.winapi_i686_pc_windows_gnu."0.4.0" = deps: f: updateFeatures f (rec {
    winapi_i686_pc_windows_gnu."0.4.0".default = (f.winapi_i686_pc_windows_gnu."0.4.0".default or true);
  }) [];


  crates.winapi_util."0.1.1" = deps: { features?(features_.winapi_util."0.1.1" deps {}) }: buildRustCrate {
    crateName = "winapi-util";
    version = "0.1.1";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "10madanla73aagbklx6y73r2g2vwq9w8a0qcghbbbpn9vfr6a95f";
    dependencies = (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."winapi_util"."0.1.1"."winapi"}" deps)
    ]) else []);
  };
  features_.winapi_util."0.1.1" = deps: f: updateFeatures f (rec {
    winapi = fold recursiveUpdate {} [
      { "${deps.winapi_util."0.1.1".winapi}"."consoleapi" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."errhandlingapi" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."fileapi" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."minwindef" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."processenv" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."std" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."winbase" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."wincon" = true; }
      { "${deps.winapi_util."0.1.1".winapi}"."winerror" = true; }
      { "${deps.winapi_util."0.1.1".winapi}".default = true; }
    ];
    winapi_util."0.1.1".default = (f.winapi_util."0.1.1".default or true);
  }) [
    (features_.winapi."${deps."winapi_util"."0.1.1"."winapi"}" deps)
  ];


  crates.winapi_x86_64_pc_windows_gnu."0.4.0" = deps: { features?(features_.winapi_x86_64_pc_windows_gnu."0.4.0" deps {}) }: buildRustCrate {
    crateName = "winapi-x86_64-pc-windows-gnu";
    version = "0.4.0";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "0n1ylmlsb8yg1v583i4xy0qmqg42275flvbc51hdqjjfjcl9vlbj";
    build = "build.rs";
  };
  features_.winapi_x86_64_pc_windows_gnu."0.4.0" = deps: f: updateFeatures f (rec {
    winapi_x86_64_pc_windows_gnu."0.4.0".default = (f.winapi_x86_64_pc_windows_gnu."0.4.0".default or true);
  }) [];


  crates.wincolor."1.0.1" = deps: { features?(features_.wincolor."1.0.1" deps {}) }: buildRustCrate {
    crateName = "wincolor";
    version = "1.0.1";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "0gr7v4krmjba7yq16071rfacz42qbapas7mxk5nphjwb042a8gvz";
    dependencies = mapFeatures features ([
      (crates."winapi"."${deps."wincolor"."1.0.1"."winapi"}" deps)
      (crates."winapi_util"."${deps."wincolor"."1.0.1"."winapi_util"}" deps)
    ]);
  };
  features_.wincolor."1.0.1" = deps: f: updateFeatures f (rec {
    winapi = fold recursiveUpdate {} [
      { "${deps.wincolor."1.0.1".winapi}"."minwindef" = true; }
      { "${deps.wincolor."1.0.1".winapi}"."wincon" = true; }
      { "${deps.wincolor."1.0.1".winapi}".default = true; }
    ];
    winapi_util."${deps.wincolor."1.0.1".winapi_util}".default = true;
    wincolor."1.0.1".default = (f.wincolor."1.0.1".default or true);
  }) [
    (features_.winapi."${deps."wincolor"."1.0.1"."winapi"}" deps)
    (features_.winapi_util."${deps."wincolor"."1.0.1"."winapi_util"}" deps)
  ];


  crates.ws2_32_sys."0.2.1" = deps: { features?(features_.ws2_32_sys."0.2.1" deps {}) }: buildRustCrate {
    crateName = "ws2_32-sys";
    version = "0.2.1";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1zpy9d9wk11sj17fczfngcj28w4xxjs3b4n036yzpy38dxp4f7kc";
    libName = "ws2_32";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."winapi"."${deps."ws2_32_sys"."0.2.1"."winapi"}" deps)
    ]);

    buildDependencies = mapFeatures features ([
      (crates."winapi_build"."${deps."ws2_32_sys"."0.2.1"."winapi_build"}" deps)
    ]);
  };
  features_.ws2_32_sys."0.2.1" = deps: f: updateFeatures f (rec {
    winapi."${deps.ws2_32_sys."0.2.1".winapi}".default = true;
    winapi_build."${deps.ws2_32_sys."0.2.1".winapi_build}".default = true;
    ws2_32_sys."0.2.1".default = (f.ws2_32_sys."0.2.1".default or true);
  }) [
    (features_.winapi."${deps."ws2_32_sys"."0.2.1"."winapi"}" deps)
    (features_.winapi_build."${deps."ws2_32_sys"."0.2.1"."winapi_build"}" deps)
  ];


  crates.yaml_rust."0.4.2" = deps: { features?(features_.yaml_rust."0.4.2" deps {}) }: buildRustCrate {
    crateName = "yaml-rust";
    version = "0.4.2";
    authors = [ "Yuheng Chen <yuhengchen@sensetime.com>" ];
    sha256 = "1bxc5hhky8rk5r8hrv4ynppsfkivq07jbj458i3h8zkhc1ca33lk";
    dependencies = mapFeatures features ([
      (crates."linked_hash_map"."${deps."yaml_rust"."0.4.2"."linked_hash_map"}" deps)
    ]);
  };
  features_.yaml_rust."0.4.2" = deps: f: updateFeatures f (rec {
    linked_hash_map."${deps.yaml_rust."0.4.2".linked_hash_map}".default = true;
    yaml_rust."0.4.2".default = (f.yaml_rust."0.4.2".default or true);
  }) [
    (features_.linked_hash_map."${deps."yaml_rust"."0.4.2"."linked_hash_map"}" deps)
  ];


}
