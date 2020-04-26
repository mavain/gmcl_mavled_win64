#![allow(clippy::missing_safety_doc)]
#![allow(non_snake_case)]

use std::fs::*;
use std::io::*;
use std::time::*;
use std::thread;

use std::os::raw::*;
use std::ffi::CString;
use glua_sys::*;

static mut ARDUINO_SERIAL_OPT: Option<std::fs::File> = None;

#[no_mangle]
pub unsafe extern "C" fn gmod13_open(L: *mut lua_State) -> c_int {

    lua_newtable!(L);

    glua_register_to_table(L, -2, "SetColor", set_mavled_color);
    glua_register_to_table(L, -2, "Connect", connect_to_mavled_arduino);

    glua_setglobal(L, "mavled");

    0
}

#[no_mangle]
pub extern "C" fn gmod13_close(_L: *mut lua_State) -> c_int {
    unsafe {
        ARDUINO_SERIAL_OPT = None;
    }
    0
}

extern "C" fn set_mavled_color(L: *mut lua_State) -> c_int {
    unsafe {

        luaL_checktype(L, 1, LUA_TNUMBER as _);
        luaL_checktype(L, 2, LUA_TNUMBER as _);
        luaL_checktype(L, 3, LUA_TNUMBER as _);

        let red = lua_tonumber(L, 1) as u8;
        let green = lua_tonumber(L, 2) as u8;
        let blue = lua_tonumber(L, 3) as u8;

        if let Some(arduino_serial) = ARDUINO_SERIAL_OPT.as_mut() {
            let _ = arduino_serial.write(format!("<COL{:02X}{:02X}{:02X}>", red, green, blue).as_bytes());
        }

    }
    0
}

extern "C" fn connect_to_mavled_arduino(_L: *mut lua_State) -> c_int {
    
    let arduino_serial = find_specific_arduino();

    match arduino_serial {
        Ok(arduino_serial) => { 

            unsafe {
                ARDUINO_SERIAL_OPT = Some(arduino_serial);
            }

        }
        Err(e) => { println!("{}", e) }
    }

    0

}

fn find_specific_arduino() -> Result<std::fs::File> {

    for com_index in 0 .. 255 {

        match OpenOptions::new().read(true).write(true).open(format!("\\\\.\\COM{}", com_index)) {
            Ok(mut f) => {

                f.write_all(b"<GETID>").unwrap();

                let handle = thread::spawn(move || {
                    let mut buffer = [0; 14];

                    let _ = f.read_exact(&mut buffer); // This disgusting syntax allows me to ignore the result completely, so no panic from unfilled buffer yay!

                    std::str::from_utf8(&buffer).unwrap().to_owned()
                });

                thread::sleep(Duration::from_millis(500));
                let arduino_id = handle.join().unwrap_or_else(|_| "UNIDENTIFIED_DEVICE".to_string());

                if arduino_id.trim() == "MAVLED_ARDUINO" {
                    return OpenOptions::new().read(true).write(true).open(format!("\\\\.\\COM{}", com_index));
                }
                
            },
            Err(_) => {
                continue;
            }
        }

    }

    Err(Error::new(ErrorKind::NotFound, "Could not find specific Arduino."))

}

fn glua_setglobal(L: *mut lua_State, lua_name: &str) {
    match CString::new(lua_name) {
        Ok(cstring_name) => {
            unsafe {
                lua_setglobal!(L, cstring_name.as_ptr());
            }
        }
        Err(e) => {
            println!("Failed to create CString! {}", e);
        }
    }
}

fn glua_register_to_table(L: *mut lua_State, table_index: i32, lua_name: &str, func: unsafe extern "C" fn(*mut lua_State) -> c_int) {
    match CString::new(lua_name) {
        Ok(cstring_name) => {
            unsafe {
                lua_pushcfunction!(L, Some(func));
                lua_setfield(L, table_index, cstring_name.as_ptr());
            }
        }
        Err(e) => {
            println!("Failed to create CString! {}", e);
        }
    }
}