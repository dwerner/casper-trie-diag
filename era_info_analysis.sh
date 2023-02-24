# c5eb2e475495c583618eb9d1b9fe2c40009cc02cfdfb503bbae6d2205891f54f 1,221,456 era 6914 oct 29
cargo run --release -- -p ../data.lmdb -d TRIE_STORE -s \
315f8a591ee32f4d80cb3b4f033b7a455d878bb4ee27ce91381475c7c7f91a43 &
# "Key::EraInfo", "EraInfo", 943567, 1753936, 6523824625

# block f39f3009deb0adaff8772a04de97aa30ce97ce605ad0fcbb028b701caf3fd013 1,026,457 era 6023 Aug 16, 2022
cargo run --release -- -p ../data.lmdb -d TRIE_STORE -s \
36cf06845e15e6dfafdaf3095a377abe8921d46ce50e02aacf56ad5c9b9d903e &
# "Key::EraInfo", "EraInfo", 832251, 1656270, 5012649074

# block  8206469327cc5172d62c92a34af0d38aaa4f3dc61e4eb346db4b87f0969ad1f0 726,458 era 4650 Apr 23, 2022
cargo run --release -- -p ../data.lmdb -d TRIE_STORE -s \
96efe0ccbe285b4110278e4a9437ae6eaaefc3148a7cc11ce0ad0dc29fbab615 &
# "Key::EraInfo", "EraInfo", 616561, 1538218, 2867008902

# block 9765138f85b7cAE2248a4468Ca1831777451ae5a68381207c7A62cDd2938675E 426,461 era 3270 Dec 29, 2021
cargo run --release -- -p ../data.lmdb -d TRIE_STORE -s \
ad507bf5abbdce9a4384ce9a34f2307d284c4a5c3a3aa3ac20371a8aebaaf6b8 &
# "Key::EraInfo", "EraInfo", 293608, 1538218, 960101223

# block 67e29092e00e86e35d6d070d1e999c7c7a0f3c9090ea48e561e1f6eb18f32110 26,465 Apr 21, 2021
cargo run --release -- -p ../data.lmdb -d TRIE_STORE -s \
9f563f8be119e55f854aa998297d54300507f57971d8524b8a9a633c8a68a906
# "Key::EraInfo", "EraInfo", 12882, 13864, 3194932

# c5eb2e475495c583618eb9d1b9fe2c40009cc02cfdfb503bbae6d2205891f54f, 1221456, 6914, "oct 29", "Key::EraInfo", "EraInfo", 943567, 1753936, 6523824625
# f39f3009deb0adaff8772a04de97aa30ce97ce605ad0fcbb028b701caf3fd013, 1026457, 6023, "Aug 16, 2022", "Key::EraInfo", "EraInfo", 832251, 1656270, 5012649074
# 8206469327cc5172d62c92a34af0d38aaa4f3dc61e4eb346db4b87f0969ad1f0, 726458, 4650, "Apr 23, 2022", "Key::EraInfo", "EraInfo", 616561, 1538218, 2867008902
# 9765138f85b7cAE2248a4468Ca1831777451ae5a68381207c7A62cDd2938675E, 426461, 3270, "Dec 29, 2021", "Key::EraInfo", "EraInfo", 293608, 1538218, 960101223
# 67e29092e00e86e35d6d070d1e999c7c7a0f3c9090ea48e561e1f6eb18f32110, 26465, 248, "Apr 21, 2021", "Key::EraInfo", "EraInfo", 12882, 13864, 3194932
