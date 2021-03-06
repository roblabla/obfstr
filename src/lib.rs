/*!
Compiletime string literal obfuscation.
*/

#![allow(incomplete_features)]
#![feature(const_fn, const_generics, const_panic)]
#![no_std]

use core::{char, fmt, mem, ptr, str};

//----------------------------------------------------------------

/// Compiletime random number generator.
///
/// Supported types are `u8`, `u16`, `u32`, `u64`, `usize`, `i8`, `i16`, `i32`, `i64`, `isize`, `bool`, `f32` and `f64`.
///
/// The integer types generate a random value in their respective range.  
/// The float types generate a random value in range of `[1.0, 2.0)`.
///
/// While the result is generated at compiletime only the integer types are available in const contexts.
///
/// Note that the seed _must_ be a uniformly distributed random `u64` value.
/// If such a value is not available, see the [`splitmix`](fn.splitmix.html) function to generate it from non uniform random value.
///
/// ```
/// const RND: i32 = obfstr::random!(u8) as i32;
/// assert!(RND >= 0 && RND <= 255);
/// ```
#[macro_export]
macro_rules! random {
	($ty:ident) => {{ const ENTROPY: u64 = $crate::entropy(file!(), line!(), column!()); $crate::random!($ty, ENTROPY) }};

	(u8, $seed:expr) => { $seed as u8 };
	(u16, $seed:expr) => { $seed as u16 };
	(u32, $seed:expr) => { $seed as u32 };
	(u64, $seed:expr) => { $seed as u64 };
	(usize, $seed:expr) => { $seed as usize };
	(i8, $seed:expr) => { $seed as i8 };
	(i16, $seed:expr) => { $seed as i16 };
	(i32, $seed:expr) => { $seed as i32 };
	(i64, $seed:expr) => { $seed as i64 };
	(isize, $seed:expr) => { $seed as isize };
	(bool, $seed:expr) => { $seed as i64 >= 0 };
	(f32, $seed:expr) => { f32::from_bits(0b0_01111111 << (f32::MANTISSA_DIGITS - 1) | ($seed as u32 >> 9)) };
	(f64, $seed:expr) => { f64::from_bits(0b0_01111111111 << (f64::MANTISSA_DIGITS - 1) | ($seed >> 12)) };
	($_:ident, $seed:expr) => { compile_error!(concat!("unsupported type: ", stringify!($_))) };
}

/// Compiletime bitmixing.
///
/// Takes an intermediate hash that may not be thoroughly mixed and increase its entropy to obtain both better distribution.
/// See [Better Bit Mixing](https://zimbry.blogspot.com/2011/09/better-bit-mixing-improving-on.html) for reference.
#[inline(always)]
pub const fn splitmix(seed: u64) -> u64 {
	let next = seed.wrapping_add(0x9e3779b97f4a7c15);
	let mut z = next;
	z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
	z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
	return z ^ (z >> 31);
}

/// Compiletime string hash.
///
/// Implemented using the [DJB2 hash function](http://www.cse.yorku.ca/~oz/hash.html#djb2).
#[inline(always)]
pub const fn hash(s: &str) -> u32 {
	let s = s.as_bytes();
	let mut result = 3581u32;
	let mut i = 0usize;
	while i < s.len() {
		result = result.wrapping_mul(33).wrapping_add(s[i] as u32);
		i += 1;
	}
	return result;
}

/// Compiletime string hash.
///
/// Helper macro guarantees compiletime evaluation of the string literal hash.
///
/// ```
/// const STRING: &str = "Hello World";
/// assert_eq!(obfstr::hash!(STRING), 1481604729);
/// ```
#[macro_export]
macro_rules! hash {
	($string:expr) => {{ const HASH: u32 = $crate::hash($string); HASH }};
}

/// Produces pseudorandom entropy given the file, line and column information.
#[doc(hidden)]
#[inline(always)]
pub const fn entropy(file: &str, line: u32, column: u32) -> u64 {
	splitmix(splitmix(splitmix(SEED ^ hash(file) as u64) ^ line as u64) ^ column as u64)
}

/// Compiletime RNG seed.
///
/// This value is derived from the environment variable `OBFSTR_SEED` and has a fixed value if absent.
/// If it changes all downstream dependents are recompiled automatically.
pub const SEED: u64 = splitmix(hash(env!("OBFSTR_SEED")) as u64);

//----------------------------------------------------------------

const XREF_SHIFT: usize = ((random!(u8) & 31) + 32) as usize;

const fn next_round(mut x: u32) -> u32 {
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	x
}

//----------------------------------------------------------------

/// Wide string literal, returns an array of words.
///
/// The type of the returned literal is `&'static [u16; LEN]`.
///
/// ```
/// let expected = &['W' as u16, 'i' as u16, 'd' as u16, 'e' as u16, 0];
/// assert_eq!(obfstr::wide!("Wide\0"), expected);
/// ```
#[macro_export]
macro_rules! wide {
	($s:literal) => { &$crate::wide::<{$crate::wide_len($s)}>($s) };
}

#[doc(hidden)]
pub const fn wide_len(s: &str) -> usize {
	let s = s.as_bytes();
	let mut len = 0usize;
	let mut i = 0usize;
	while i < s.len() {
		let chr;
		if s[i] & 0x80 == 0x00 {
			chr = s[i] as u32;
			i += 1;
		}
		else if s[i] & 0xe0 == 0xc0 {
			chr = (s[i] as u32 & 0x1f) << 6 | (s[i + 1] as u32 & 0x3f);
			i += 2;
		}
		else if s[i] & 0xf0 == 0xe0 {
			chr = (s[i] as u32 & 0x0f) << 12 | (s[i + 1] as u32 & 0x3f) << 6 | (s[i + 2] as u32 & 0x3f);
			i += 3;
		}
		else if s[i] & 0xf8 == 0xf0 {
			chr = (s[i] as u32 & 0x07) << 18 | (s[i + 1] as u32 & 0x3f) << 12 | (s[i + 2] as u32 & 0x3f) << 6 | (s[i + 3] as u32 & 0x3f);
			i += 4;
		}
		else {
			unimplemented!()
		};
		len += if chr >= 0x10000 { 2 } else { 1 };
	}
	return len;
}

#[doc(hidden)]
pub const fn wide<const LEN: usize>(s: &str) -> [u16; LEN] {
	let s = s.as_bytes();
	let mut data = [0u16; LEN];
	let mut i = 0usize;
	let mut j = 0usize;
	while i < s.len() {
		let chr;
		if s[i] & 0x80 == 0x00 {
			chr = s[i] as u32;
			i += 1;
		}
		else if s[i] & 0xe0 == 0xc0 {
			chr = (s[i] as u32 & 0x1f) << 6 | (s[i + 1] as u32 & 0x3f);
			i += 2;
		}
		else if s[i] & 0xf0 == 0xe0 {
			chr = (s[i] as u32 & 0x0f) << 12 | (s[i + 1] as u32 & 0x3f) << 6 | (s[i + 2] as u32 & 0x3f);
			i += 3;
		}
		else if s[i] & 0xf8 == 0xf0 {
			chr = (s[i] as u32 & 0x07) << 18 | (s[i + 1] as u32 & 0x3f) << 12 | (s[i + 2] as u32 & 0x3f) << 6 | (s[i + 3] as u32 & 0x3f);
			i += 4;
		}
		else {
			unimplemented!()
		};
		if chr >= 0x10000 {
			data[j + 0] = (0xD800 + (chr - 0x10000) / 0x400) as u16;
			data[j + 1] = (0xDC00 + (chr - 0x10000) % 0x400) as u16;
			j += 2;
		}
		else {
			data[j] = chr as u16;
			j += 1;
		}
	}
	return data;
}

//----------------------------------------------------------------

/// Obfuscated string constant data.
///
/// This type represents the data baked in the binary and holds the key and obfuscated string.
#[repr(C)]
pub struct ObfString<A> {
	key: u32,
	data: A,
}

/// Deobfuscated string buffer.
#[repr(transparent)]
pub struct ObfBuffer<A>(A);

impl<A> AsRef<A> for ObfBuffer<A> {
	#[inline] fn as_ref(&self) -> &A { &self.0 }
}

//----------------------------------------------------------------
// Byte strings.

impl<const LEN: usize> ObfString<[u8; LEN]> {
	/// Deobfuscates the string and returns the buffer.
	///
	/// The `x` argument should be a compiletime random 16-bit value.
	/// It is used to obfuscate the underlying call to the decrypt routine.
	#[inline(always)]
	pub fn deobfuscate(&self, x: usize) -> ObfBuffer<[u8; LEN]> {
		unsafe {
			let mut buffer = mem::MaybeUninit::<[u8; LEN]>::uninit();

			let dest = buffer.as_mut_ptr() as *mut u8;
			let src = self.data.as_ptr().wrapping_offset(-((LEN * XREF_SHIFT) as isize));

			let f: unsafe fn(*mut u8, *const u8, usize) = mem::transmute(ptr::read_volatile(&(decryptbuf as usize + x)) - x);
			f(dest, src, LEN);

			ObfBuffer(buffer.assume_init())
		}
	}
	#[doc(hidden)]
	pub const fn obfuscate(key: u32, string: &str) -> ObfString<[u8; LEN]> {
		let string = string.as_bytes();
		let mut data = [0u8; LEN];
		let mut round_key = key;
		let mut i = 0usize;
		while i < string.len() {
			round_key = next_round(round_key);
			data[i] = string[i] ^ round_key as u8;
			i += 1;
		}
		ObfString { key, data }
	}
	#[doc(hidden)]
	#[inline(always)]
	pub fn eq(&self, s: &str, x: usize) -> bool {
		if LEN != s.len() {
			return false;
		}
		unsafe {
			let obfstr = self.data.as_ptr().wrapping_offset(-((LEN * XREF_SHIFT) as isize));
			let f: unsafe fn(*const u8, *const u8, usize) -> bool = mem::transmute(ptr::read_volatile(&(decrypteq as usize + x)) - x);
			f(obfstr, s.as_ptr(), LEN)
		}
	}
}

#[inline(never)]
unsafe fn decryptbuf(dest: *mut u8, src: *const u8, len: usize) {
	let src = src.wrapping_offset((len * XREF_SHIFT) as isize);
	let mut key = *(src as *const u32).offset(-1);
	for i in 0..len {
		key = next_round(key);
		*dest.offset(i as isize) = *src.offset(i as isize) ^ key as u8;
	}
}
#[inline(never)]
unsafe fn decrypteq(obfstr: *const u8, clearstr: *const u8, len: usize) -> bool {
	let obfstr = obfstr.wrapping_offset((len * XREF_SHIFT) as isize);
	let mut key = *(obfstr as *const u32).offset(-1);
	for i in 0..len {
		key = next_round(key);
		if *clearstr.offset(i as isize) != *obfstr.offset(i as isize) ^ key as u8 {
			return false;
		}
	}
	true
}

impl<const LEN: usize> ObfBuffer<[u8; LEN]> {
	#[inline]
	pub const fn as_slice(&self) -> &[u8] {
		&self.0
	}
	#[inline]
	pub fn as_str(&self) -> &str {
		// This should be safe as it can only be constructed from a string literal...
		#[cfg(debug_assertions)]
		return str::from_utf8(&self.0).unwrap();
		#[cfg(not(debug_assertions))]
		return unsafe { str::from_utf8_unchecked(&self.0) };
	}
	// For use with serde's stupid 'static limitations...
	#[cfg(feature = "unsafe_static_str")]
	#[inline]
	pub fn unsafe_as_static_str(&self) -> &'static str {
		unsafe { &*(self.as_str() as *const str) }
	}
}
impl<const LEN: usize> fmt::Debug for ObfBuffer<[u8; LEN]> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.as_str().fmt(f)
	}
}

//----------------------------------------------------------------
// Word strings.

impl<const LEN: usize> ObfString<[u16; LEN]> {
	/// Deobfuscates the string and returns the buffer.
	///
	/// The `x` argument should be a compiletime random 16-bit value.
	/// It is used to obfuscate the underlying call to the decrypt routine.
	#[inline(always)]
	pub fn deobfuscate(&self, x: usize) -> ObfBuffer<[u16; LEN]> {
		unsafe {
			let mut buffer = mem::MaybeUninit::<[u16; LEN]>::uninit();

			let dest = buffer.as_mut_ptr() as *mut u16;
			let src = (&self.data as *const _ as *const u16).wrapping_offset(-((LEN * XREF_SHIFT) as isize));

			let f: unsafe fn(*mut u16, *const u16, usize) = mem::transmute(ptr::read_volatile(&(wdecryptbuf as usize + x)) - x);
			f(dest, src, LEN);

			ObfBuffer(buffer.assume_init())
		}
	}
	#[doc(hidden)]
	pub const fn obfuscate(key: u32, string: &str) -> ObfString<[u16; LEN]> {
		let string = wide::<LEN>(string);
		let mut data = [0u16; LEN];
		let mut round_key = key;
		let mut i = 0usize;
		while i < string.len() {
			round_key = next_round(round_key);
			data[i] = string[i] as u16 ^ round_key as u16;
			i += 1;
		}
		ObfString { key, data }
	}
	#[doc(hidden)]
	#[inline(always)]
	pub fn eq(&self, s: &[u16], x: usize) -> bool {
		if LEN != s.len() {
			return false;
		}
		unsafe {
			let obfstr = self.data.as_ptr().wrapping_offset(-((LEN * XREF_SHIFT) as isize));
			let f: unsafe fn(*const u16, *const u16, usize) -> bool = mem::transmute(ptr::read_volatile(&(wdecrypteq as usize + x)) - x);
			f(obfstr, s.as_ptr(), LEN)
		}
	}
}
impl<const LEN: usize> fmt::Debug for ObfString<[u16; LEN]> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.deobfuscate(random!(usize) & 0xffff).fmt(f)
	}
}

#[inline(never)]
unsafe fn wdecryptbuf(dest: *mut u16, src: *const u16, len: usize) {
	let src = src.wrapping_offset((len * XREF_SHIFT) as isize);
	let mut key = *(src as *const u32).offset(-1);
	for i in 0..len {
		key = next_round(key);
		*dest.offset(i as isize) = *src.offset(i as isize) ^ key as u16;
	}
}
#[inline(never)]
unsafe fn wdecrypteq(obfstr: *const u16, clearstr: *const u16, len: usize) -> bool {
	let obfstr = obfstr.wrapping_offset((len * XREF_SHIFT) as isize);
	let mut key = *(obfstr as *const u32).offset(-1);
	for i in 0..len {
		key = next_round(key);
		if *clearstr.offset(i as isize) != *obfstr.offset(i as isize) ^ key as u16 {
			return false;
		}
	}
	true
}

impl<const LEN: usize> ObfBuffer<[u16; LEN]> {
	#[inline]
	pub const fn as_slice(&self) -> &[u16] {
		&self.0
	}
}
impl<const LEN: usize> fmt::Debug for ObfBuffer<[u16; LEN]> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use fmt::Write;
		f.write_str("\"")?;
		for chr in char::decode_utf16(self.as_slice().iter().cloned()) {
			f.write_char(chr.unwrap_or(char::REPLACEMENT_CHARACTER))?;
		}
		f.write_str("\"")
	}
}

//----------------------------------------------------------------

/// Compiletime string literal obfuscation.
///
/// Returns a borrowed temporary and may not escape the statement it was used in.
///
/// Prefix the string literal with `L` to get an UTF-16 obfuscated string.
///
/// ```
/// assert_eq!(obfstr::obfstr!("Hello 🌍"), "Hello 🌍");
/// ```
#[macro_export]
macro_rules! obfstr {
	($s:literal) => { $crate::obflocal!($s).as_str() };
	(L$s:literal) => { $crate::obflocal!(L$s).as_ref() };
}

/// Compiletime string literal obfuscation.
///
/// Returns the deobfuscated [`ObfBuffer`](struct.ObfBuffer.html) for assignment to local variable.
///
/// Prefix the string literal with `L` to get an UTF-16 obfuscated string.
///
/// ```
/// let str_buf = obfstr::obflocal!("Hello 🌍");
/// assert_eq!(str_buf.as_str(), "Hello 🌍");
/// ```
#[macro_export]
macro_rules! obflocal {
	($s:literal) => { $crate::obfconst!($s).deobfuscate($crate::random!(usize) & 0xffff) };
	(L$s:literal) => { $crate::obfconst!(L$s).deobfuscate($crate::random!(usize) & 0xffff) };
}

/// Compiletime string literal obfuscation.
///
/// Returns the obfuscated [`ObfString`](struct.ObfString.html) for use in constant expressions.
///
/// Prefix the string literal with `L` to get an UTF-16 obfuscated string.
///
/// ```
/// static GSTR: obfstr::ObfString<[u8; 10]> = obfstr::obfconst!("Hello 🌍");
/// assert_eq!(GSTR.deobfuscate(0).as_str(), "Hello 🌍");
/// ```
#[macro_export]
macro_rules! obfconst {
	($s:literal) => {{ const STRING: $crate::ObfString<[u8; {$s.len()}]> = $crate::ObfString::<[u8; {$s.len()}]>::obfuscate($crate::random!(u32), $s); STRING }};
	(L$s:literal) => {{ const STRING: $crate::ObfString<[u16; {$crate::wide_len($s)}]> = $crate::ObfString::<[u16; {$crate::wide_len($s)}]>::obfuscate($crate::random!(u32), $s); STRING }};
}

/// Check if string equals specific string literal.
///
/// This does not need to decrypt the string before comparison and the comparison is not constant-time.
///
/// ```
/// let e = "Hello 🌍";
/// assert!(obfstr::obfeq!(e, "Hello 🌍"));
/// ```
#[macro_export]
macro_rules! obfeq {
	($e:expr, $s:literal) => { $crate::obfconst!($s).eq(&$e, $crate::random!(usize) & 0xffff) };
	($e:expr, L$s:literal) => { $crate::obfconst!(L$s).eq($e, $crate::random!(usize) & 0xffff) };
}
