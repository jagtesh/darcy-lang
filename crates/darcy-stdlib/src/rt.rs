use std::fmt::Debug;
use std::sync::Arc;

pub fn core_clone<T: Clone>(val: &T) -> T {
    val.clone()
}

pub trait FromInt {
    fn from_int(val: i64) -> Self;
}

pub fn from_int<T: FromInt>(val: i64) -> T {
    T::from_int(val)
}

macro_rules! impl_from_int {
    ($($t:ty),+ $(,)?) => {
        $(
            impl FromInt for $t {
                fn from_int(val: i64) -> Self {
                    val as $t
                }
            }
        )+
    };
}

impl_from_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

pub fn math_gt<T: PartialOrd>(a: T, b: T) -> bool {
    a > b
}

pub fn math_lt<T: PartialOrd>(a: T, b: T) -> bool {
    a < b
}

pub fn math_eq<T: PartialEq>(a: T, b: T) -> bool {
    a == b
}

pub fn math_exp(x: f64) -> f64 {
    x.exp()
}

pub fn math_abs<T>(x: T) -> T
where
    T: PartialOrd + Default + Clone + std::ops::Sub<Output = T>,
{
    let zero = T::default();
    if x < zero.clone() {
        zero - x
    } else {
        x
    }
}

pub fn math_min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

pub fn math_max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}

pub fn math_clamp<T: PartialOrd>(x: T, lo: T, hi: T) -> T {
    math_min(math_max(x, lo), hi)
}

pub fn vec_new<T>() -> Arc<Vec<T>> {
    Arc::new(Vec::new())
}

pub fn vec_len<T>(v: &Arc<Vec<T>>) -> usize {
    v.len()
}

pub fn vec_get<T: Clone>(v: &Arc<Vec<T>>, idx: usize) -> T {
    v[idx].clone()
}

pub fn vec_set<T: Clone>(mut v: Arc<Vec<T>>, idx: usize, val: T) -> Arc<Vec<T>> {
    let slot = Arc::make_mut(&mut v);
    slot[idx] = val;
    v
}

pub fn vec_push<T: Clone>(mut v: Arc<Vec<T>>, val: T) -> Arc<Vec<T>> {
    let slot = Arc::make_mut(&mut v);
    slot.push(val);
    v
}

pub fn vec_repeat<T: Clone>(val: T, n: usize) -> Arc<Vec<T>> {
    Arc::new(vec![val; n])
}

pub fn vec_map<T, U, F>(mut f: F, v: Arc<Vec<T>>) -> Arc<Vec<U>>
where
    F: FnMut(T) -> U,
    T: Clone,
{
    Arc::new(v.iter().cloned().map(|x| f(x)).collect())
}

pub fn vec_map2<A, B, C, F>(mut f: F, v1: Arc<Vec<A>>, v2: Arc<Vec<B>>) -> Arc<Vec<C>>
where
    F: FnMut(A, B) -> C,
    A: Clone,
    B: Clone,
{
    Arc::new(
        v1.iter()
            .cloned()
            .zip(v2.iter().cloned())
            .map(|(a, b)| f(a, b))
            .collect(),
    )
}

pub fn vec_fold<A, B, F>(mut f: F, init: A, v: Arc<Vec<B>>) -> A
where
    F: FnMut(A, B) -> A,
    B: Clone,
{
    v.iter().cloned().fold(init, |acc, x| f(acc, x))
}

pub fn vec_take<T: Clone>(v: Arc<Vec<T>>, n: usize) -> Arc<Vec<T>> {
    Arc::new(v.iter().take(n).cloned().collect())
}

pub fn vec_range(n: usize) -> Arc<Vec<usize>> {
    Arc::new((0..n).collect())
}

pub fn string_len(s: &String) -> usize {
    s.len()
}

pub fn string_is_empty(s: &String) -> bool {
    s.is_empty()
}

pub fn string_trim(s: &String) -> String {
    s.trim().to_string()
}

pub fn string_split(s: &String, sep: &String) -> Arc<Vec<String>> {
    Arc::new(s.split(sep).map(|part| part.to_string()).collect())
}

pub fn string_join(items: Arc<Vec<String>>, sep: &String) -> String {
    items.join(sep)
}

pub fn cast_usize<T: IntoUsize>(v: T) -> usize {
    v.into_usize()
}

pub trait IntoUsize {
    fn into_usize(self) -> usize;
}

impl IntoUsize for usize {
    fn into_usize(self) -> usize {
        self
    }
}

impl IntoUsize for isize {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for i64 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for i32 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for u64 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for u32 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for i128 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for u128 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for i16 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for u16 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for i8 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for u8 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for bool {
    fn into_usize(self) -> usize {
        self as usize
    }
}

pub fn option_some<T>(val: T) -> Option<T> {
    Some(val)
}

pub fn option_none<T>() -> Option<T> {
    None
}

pub fn option_is_some<T>(val: Option<T>) -> bool {
    val.is_some()
}

pub fn option_is_none<T>(val: Option<T>) -> bool {
    val.is_none()
}

pub fn option_unwrap<T>(val: Option<T>) -> T {
    val.unwrap()
}

pub fn option_unwrap_or<T>(val: Option<T>, fallback: T) -> T {
    val.unwrap_or(fallback)
}

pub fn result_ok<T, E>(val: T) -> Result<T, E> {
    Ok(val)
}

pub fn result_err<T, E>(err: E) -> Result<T, E> {
    Err(err)
}

pub fn result_is_ok<T, E>(val: Result<T, E>) -> bool {
    val.is_ok()
}

pub fn result_is_err<T, E>(val: Result<T, E>) -> bool {
    val.is_err()
}

pub fn result_unwrap<T, E: Debug>(val: Result<T, E>) -> T {
    val.unwrap()
}

pub fn result_unwrap_or<T, E>(val: Result<T, E>, fallback: T) -> T {
    val.unwrap_or(fallback)
}

pub fn fmt_format<T: Debug>(val: T) -> String {
    format!("{:?}", val)
}

pub fn fmt_pretty<T: Debug>(val: T) -> String {
    format!("{:#?}", val)
}

pub fn fmt_print(s: String) {
    print!("{}", s);
}

pub fn fmt_println(s: String) {
    println!("{}", s);
}
