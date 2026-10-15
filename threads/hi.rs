#[unsafe(no_mangle)]
extern "C" fn rust_hello() {
kprintln!("Hello from hand written rust");
for i in 0..5{
    for j in 0..i {
        kprint!("*");
    }
    kprintln!();
}
kprintln!("Testing rust's runtime bounds check");
let arr = [10, 20, 30, 40, 50];

let i = core::hint::black_box(100); //so compiler doesn't flag on the dot

kprintln!("Value: {}", arr[i]); //Test our runtime check
}