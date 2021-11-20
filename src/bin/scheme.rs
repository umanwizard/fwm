use std::{
    ffi::{c_void, CStr},
    ptr::null_mut,
};

use fwm::scheme::{Deserializer, Serializer, SCM_UNSPECIFIED};
use rust_guile::{scm_c_define_gsubr, scm_shell, scm_with_guile, SCM};
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
enum Inner {
    InnerUnit,
    InnerTuple(u64, u64),
    InnerStruct { foo: String, bar: u64 },
}
#[derive(Serialize, Deserialize, Debug)]
struct MyStruct {
    foo: u64,
    bar: (String, String),
    quux: Vec<u8>,
    xyzzy: Option<i64>,
    derp: Inner,
    // #[serde(with = "serde_bytes")]
    syzygy: Vec<u8>,
}

extern "C" fn get_serialized() -> SCM {
    let my = MyStruct {
        foo: 42,
        bar: ("hello".to_string(), "world".to_string()),
        quux: b"Brennan\0".to_vec(),
        xyzzy: Some(69),
        derp: Inner::InnerStruct { foo: "Brennan\0 was here!".to_string(), bar: 420 },
        syzygy: b"was here\0".to_vec(),
    };
    let s = Serializer::default();
    my.serialize(s).unwrap()
}

extern "C" fn print_deserialized(scm: SCM) -> SCM {
    let d = Deserializer { scm };
    let s = MyStruct::deserialize(d).unwrap();
    println!("{:?}", s);
    SCM_UNSPECIFIED
}

unsafe extern "C" fn scheme_setup(_data: *mut c_void) -> *mut c_void {
    let c = CStr::from_bytes_with_nul(b"get-serialized\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 0, 0, 0, get_serialized as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"print-deserialized\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, print_deserialized as *mut c_void);
    null_mut()
}

fn main() {
    unsafe {
        scm_with_guile(Some(scheme_setup), null_mut());
        scm_shell(0, null_mut());
    }
}
