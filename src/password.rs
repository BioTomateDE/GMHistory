/// Password for all GM:S 1 upgrade ZIPs.
///
/// Datamined from GameMaker-Studio.exe lol
pub const GMS1_PASSWORD: &str = "12#_p@o3w$ir_ADD-_$#";

/// Generates a GMS 2 ZIP password for a given runtime (UUIDv4 + ".zip").
pub fn generate_password(filename_or_url: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let filename = get_filename(filename_or_url);

    if let Some(stem) = filename.strip_suffix(".zip") {
        if let Some(bad_char) = stem.chars().find(|&c| c != '-' && !c.is_ascii_hexdigit()) {
            eprintln!("warning: filename {filename:?} contains atypical character {bad_char:?}");
        }
    } else {
        eprintln!("warning: filename {filename:?} does not end with .zip");
    }

    let input = format!("MRJA{filename}PHMD");
    let hashed: [u8; 16] = produce(input.as_bytes());
    STANDARD.encode(hashed)
}

fn get_filename(filename_or_url: &str) -> &str {
    match filename_or_url.rfind('/') {
        Some(idx) => &filename_or_url[idx + 1..],
        None => filename_or_url,
    }
}

// WARNING: Everything beyond here is slopcoded.

// Faithful port of Org.BouncyCastle.Crypto.Digests.{GeneralDigest, MD5Digest} (bc-csharp),
// including the fact that ProcessBlock() resets xOff to 0 but does NOT clear the X[16]
// word buffer. Calling Finish() and then DoFinal() (which calls Finish() again) reuses
// stale words left over from the first finalization, producing a non-standard digest.

fn produce(input: &[u8]) -> [u8; 16] {
    let mut d = BcMd5::new();
    d.block_update(input);
    d.finish();
    // this line reproduces the BouncyCastle bug
    // (comment it out for regular md5)
    d.finish();
    d.digest_bytes()
}

struct BcMd5 {
    // GeneralDigest state
    x_buf: [u8; 4],
    x_buf_off: usize,
    byte_count: u64,

    // MD5Digest state
    h1: u32,
    h2: u32,
    h3: u32,
    h4: u32,
    x: [u32; 16], // NOTE: never cleared by ProcessBlock, only xOff is reset
    x_off: usize,
}

impl BcMd5 {
    fn new() -> Self {
        let mut d = BcMd5 {
            x_buf: [0; 4],
            x_buf_off: 0,
            byte_count: 0,
            h1: 0x67452301,
            h2: 0xefcdab89,
            h3: 0x98badcfe,
            h4: 0x10325476,
            x: [0; 16],
            x_off: 0,
        };
        d.reset_full();
        d
    }

    // GeneralDigest.Reset() + MD5Digest.Reset() combined (used only at construction / after DoFinal)
    fn reset_full(&mut self) {
        self.byte_count = 0;
        self.x_buf_off = 0;
        self.x_buf = [0; 4];

        self.h1 = 0x67452301;
        self.h2 = 0xefcdab89;
        self.h3 = 0x98badcfe;
        self.h4 = 0x10325476;
        self.x_off = 0;
        self.x = [0; 16]; // Reset() DOES clear X (unlike ProcessBlock)
    }

    fn update_byte(&mut self, input: u8) {
        self.x_buf[self.x_buf_off] = input;
        self.x_buf_off += 1;
        if self.x_buf_off == self.x_buf.len() {
            self.process_word(&self.x_buf.clone(), 0);
            self.x_buf_off = 0;
        }
        self.byte_count += 1;
    }

    fn block_update(&mut self, input: &[u8]) {
        let length = input.len();
        let mut i = 0usize;

        if self.x_buf_off != 0 {
            while i < length {
                self.x_buf[self.x_buf_off] = input[i];
                self.x_buf_off += 1;
                i += 1;
                if self.x_buf_off == 4 {
                    self.process_word(&self.x_buf.clone(), 0);
                    self.x_buf_off = 0;
                    break;
                }
            }
        }

        // process whole words
        if length >= 3 {
            let limit = length - 3;
            while i < limit {
                self.process_word(input, i);
                i += 4;
            }
        }

        // remainder
        while i < length {
            self.x_buf[self.x_buf_off] = input[i];
            self.x_buf_off += 1;
            i += 1;
        }

        self.byte_count += length as u64;
    }

    // MD5Digest.ProcessWord
    fn process_word(&mut self, input: &[u8], in_off: usize) {
        self.x[self.x_off] = u32::from_le_bytes([
            input[in_off],
            input[in_off + 1],
            input[in_off + 2],
            input[in_off + 3],
        ]);
        self.x_off += 1;
        if self.x_off == 16 {
            self.process_block();
        }
    }

    // MD5Digest.ProcessLength
    fn process_length(&mut self, bit_length: u64) {
        if self.x_off > 14 {
            if self.x_off == 15 {
                self.x[15] = 0;
            }
            self.process_block();
        }
        for i in self.x_off..14 {
            self.x[i] = 0;
        }
        self.x[14] = bit_length as u32;
        self.x[15] = (bit_length >> 32) as u32;
    }

    // GeneralDigest.Finish() -- PUBLIC, callable directly (this is the bug vector)
    fn finish(&mut self) {
        let bit_length = self.byte_count << 3;
        self.update_byte(0x80);
        while self.x_buf_off != 0 {
            self.update_byte(0x00);
        }
        self.process_length(bit_length);
        self.process_block();
    }

    // MD5Digest.ProcessBlock -- standard MD5 compression. Crucially: only xOff is reset,
    // X[] itself is left with whatever stale words remain.
    fn process_block(&mut self) {
        fn f(u: u32, v: u32, w: u32) -> u32 {
            (u & v) | (!u & w)
        }
        fn g(u: u32, v: u32, w: u32) -> u32 {
            (u & w) | (v & !w)
        }
        fn h(u: u32, v: u32, w: u32) -> u32 {
            u ^ v ^ w
        }
        fn k(u: u32, v: u32, w: u32) -> u32 {
            v ^ (u | !w)
        }

        let x = &self.x;
        let mut a = self.h1;
        let mut b = self.h2;
        let mut c = self.h3;
        let mut d = self.h4;

        macro_rules! ff {
            ($a:expr,$b:expr,$c:expr,$d:expr,$xk:expr,$s:expr,$t:expr) => {
                $a = ($a
                    .wrapping_add(f($b, $c, $d))
                    .wrapping_add(x[$xk])
                    .wrapping_add($t))
                .rotate_left($s)
                .wrapping_add($b);
            };
        }
        macro_rules! gg {
            ($a:expr,$b:expr,$c:expr,$d:expr,$xk:expr,$s:expr,$t:expr) => {
                $a = ($a
                    .wrapping_add(g($b, $c, $d))
                    .wrapping_add(x[$xk])
                    .wrapping_add($t))
                .rotate_left($s)
                .wrapping_add($b);
            };
        }
        macro_rules! hh {
            ($a:expr,$b:expr,$c:expr,$d:expr,$xk:expr,$s:expr,$t:expr) => {
                $a = ($a
                    .wrapping_add(h($b, $c, $d))
                    .wrapping_add(x[$xk])
                    .wrapping_add($t))
                .rotate_left($s)
                .wrapping_add($b);
            };
        }
        macro_rules! kk {
            ($a:expr,$b:expr,$c:expr,$d:expr,$xk:expr,$s:expr,$t:expr) => {
                $a = ($a
                    .wrapping_add(k($b, $c, $d))
                    .wrapping_add(x[$xk])
                    .wrapping_add($t))
                .rotate_left($s)
                .wrapping_add($b);
            };
        }

        // Round 1
        ff!(a, b, c, d, 0, 7, 0xd76aa478u32);
        ff!(d, a, b, c, 1, 12, 0xe8c7b756u32);
        ff!(c, d, a, b, 2, 17, 0x242070dbu32);
        ff!(b, c, d, a, 3, 22, 0xc1bdceeeu32);
        ff!(a, b, c, d, 4, 7, 0xf57c0fafu32);
        ff!(d, a, b, c, 5, 12, 0x4787c62au32);
        ff!(c, d, a, b, 6, 17, 0xa8304613u32);
        ff!(b, c, d, a, 7, 22, 0xfd469501u32);
        ff!(a, b, c, d, 8, 7, 0x698098d8u32);
        ff!(d, a, b, c, 9, 12, 0x8b44f7afu32);
        ff!(c, d, a, b, 10, 17, 0xffff5bb1u32);
        ff!(b, c, d, a, 11, 22, 0x895cd7beu32);
        ff!(a, b, c, d, 12, 7, 0x6b901122u32);
        ff!(d, a, b, c, 13, 12, 0xfd987193u32);
        ff!(c, d, a, b, 14, 17, 0xa679438eu32);
        ff!(b, c, d, a, 15, 22, 0x49b40821u32);

        // Round 2
        gg!(a, b, c, d, 1, 5, 0xf61e2562u32);
        gg!(d, a, b, c, 6, 9, 0xc040b340u32);
        gg!(c, d, a, b, 11, 14, 0x265e5a51u32);
        gg!(b, c, d, a, 0, 20, 0xe9b6c7aau32);
        gg!(a, b, c, d, 5, 5, 0xd62f105du32);
        gg!(d, a, b, c, 10, 9, 0x02441453u32);
        gg!(c, d, a, b, 15, 14, 0xd8a1e681u32);
        gg!(b, c, d, a, 4, 20, 0xe7d3fbc8u32);
        gg!(a, b, c, d, 9, 5, 0x21e1cde6u32);
        gg!(d, a, b, c, 14, 9, 0xc33707d6u32);
        gg!(c, d, a, b, 3, 14, 0xf4d50d87u32);
        gg!(b, c, d, a, 8, 20, 0x455a14edu32);
        gg!(a, b, c, d, 13, 5, 0xa9e3e905u32);
        gg!(d, a, b, c, 2, 9, 0xfcefa3f8u32);
        gg!(c, d, a, b, 7, 14, 0x676f02d9u32);
        gg!(b, c, d, a, 12, 20, 0x8d2a4c8au32);

        // Round 3
        hh!(a, b, c, d, 5, 4, 0xfffa3942u32);
        hh!(d, a, b, c, 8, 11, 0x8771f681u32);
        hh!(c, d, a, b, 11, 16, 0x6d9d6122u32);
        hh!(b, c, d, a, 14, 23, 0xfde5380cu32);
        hh!(a, b, c, d, 1, 4, 0xa4beea44u32);
        hh!(d, a, b, c, 4, 11, 0x4bdecfa9u32);
        hh!(c, d, a, b, 7, 16, 0xf6bb4b60u32);
        hh!(b, c, d, a, 10, 23, 0xbebfbc70u32);
        hh!(a, b, c, d, 13, 4, 0x289b7ec6u32);
        hh!(d, a, b, c, 0, 11, 0xeaa127fau32);
        hh!(c, d, a, b, 3, 16, 0xd4ef3085u32);
        hh!(b, c, d, a, 6, 23, 0x04881d05u32);
        hh!(a, b, c, d, 9, 4, 0xd9d4d039u32);
        hh!(d, a, b, c, 12, 11, 0xe6db99e5u32);
        hh!(c, d, a, b, 15, 16, 0x1fa27cf8u32);
        hh!(b, c, d, a, 2, 23, 0xc4ac5665u32);

        // Round 4
        kk!(a, b, c, d, 0, 6, 0xf4292244u32);
        kk!(d, a, b, c, 7, 10, 0x432aff97u32);
        kk!(c, d, a, b, 14, 15, 0xab9423a7u32);
        kk!(b, c, d, a, 5, 21, 0xfc93a039u32);
        kk!(a, b, c, d, 12, 6, 0x655b59c3u32);
        kk!(d, a, b, c, 3, 10, 0x8f0ccc92u32);
        kk!(c, d, a, b, 10, 15, 0xffeff47du32);
        kk!(b, c, d, a, 1, 21, 0x85845dd1u32);
        kk!(a, b, c, d, 8, 6, 0x6fa87e4fu32);
        kk!(d, a, b, c, 15, 10, 0xfe2ce6e0u32);
        kk!(c, d, a, b, 6, 15, 0xa3014314u32);
        kk!(b, c, d, a, 13, 21, 0x4e0811a1u32);
        kk!(a, b, c, d, 4, 6, 0xf7537e82u32);
        kk!(d, a, b, c, 11, 10, 0xbd3af235u32);
        kk!(c, d, a, b, 2, 15, 0x2ad7d2bbu32);
        kk!(b, c, d, a, 9, 21, 0xeb86d391u32);

        self.h1 = self.h1.wrapping_add(a);
        self.h2 = self.h2.wrapping_add(b);
        self.h3 = self.h3.wrapping_add(c);
        self.h4 = self.h4.wrapping_add(d);

        self.x_off = 0; // X[] is deliberately NOT cleared here -- this is the bug vector
    }

    fn digest_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.h1.to_le_bytes());
        out[4..8].copy_from_slice(&self.h2.to_le_bytes());
        out[8..12].copy_from_slice(&self.h3.to_le_bytes());
        out[12..16].copy_from_slice(&self.h4.to_le_bytes());
        out
    }
}
