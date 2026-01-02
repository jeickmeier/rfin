use finstack_wasm::Bond;
use wasm_bindgen::JsCast;

fn main() {
    let b = Bond::fixed_semiannual("bond1", ...);
    let v: wasm_bindgen::JsValue = b.into();
    let b2 = v.dyn_ref::<Bond>();
}
