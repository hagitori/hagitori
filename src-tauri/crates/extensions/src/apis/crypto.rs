//! QuickJS crypto API (hashing, encoding utilities).

use rquickjs::{Ctx, Function, Object};

pub fn register<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    let crypto_obj = Object::new(ctx.clone())?;

    // crypto.md5(input) -> hex string
    crypto_obj.set(
        "md5",
        Function::new(ctx.clone(), |input: String| -> String {
            use md5::{Digest, Md5};
            let mut hasher = Md5::new();
            hasher.update(input.as_bytes());
            format!("{:x}", hasher.finalize())
        })?,
    )?;

    // crypto.sha256(input) -> hex string
    crypto_obj.set(
        "sha256",
        Function::new(ctx.clone(), |input: String| -> String {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(input.as_bytes());
            format!("{:x}", hasher.finalize())
        })?,
    )?;

    // crypto.sha512(input) -> hex string
    crypto_obj.set(
        "sha512",
        Function::new(ctx.clone(), |input: String| -> String {
            use sha2::{Digest, Sha512};
            let mut hasher = Sha512::new();
            hasher.update(input.as_bytes());
            format!("{:x}", hasher.finalize())
        })?,
    )?;

    // crypto.hmacSha256(key, msg) -> hex string
    crypto_obj.set(
        "hmacSha256",
        Function::new(ctx.clone(), |key: String, msg: String| -> rquickjs::Result<String> {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            let mut mac = HmacSha256::new_from_slice(key.as_bytes())
                .map_err(|e| rquickjs::Error::new_from_js_message("crypto", "hmac", &format!("{e}")))?;
            mac.update(msg.as_bytes());
            Ok(format!("{:x}", mac.finalize().into_bytes()))
        })?,
    )?;

    // crypto.hmacSha512(key, msg) -> hex string
    crypto_obj.set(
        "hmacSha512",
        Function::new(ctx.clone(), |key: String, msg: String| -> rquickjs::Result<String> {
            use hmac::{Hmac, Mac};
            use sha2::Sha512;
            type HmacSha512 = Hmac<Sha512>;
            let mut mac = HmacSha512::new_from_slice(key.as_bytes())
                .map_err(|e| rquickjs::Error::new_from_js_message("crypto", "hmac", &format!("{e}")))?;
            mac.update(msg.as_bytes());
            Ok(format!("{:x}", mac.finalize().into_bytes()))
        })?,
    )?;

    // crypto.randomUUID() -> UUID v4 string
    crypto_obj.set(
        "randomUUID",
        Function::new(ctx.clone(), || -> rquickjs::Result<String> {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes)
                .map_err(|e| rquickjs::Error::new_from_js_message("crypto", "randomUUID", &format!("getrandom failed: {e}")))?;
            bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
            bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 1
            Ok(format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                bytes[0], bytes[1], bytes[2], bytes[3],
                bytes[4], bytes[5],
                bytes[6], bytes[7],
                bytes[8], bytes[9],
                bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
            ))
        })?,
    )?;

    // crypto.randomBytes(n) -> number[]
    crypto_obj.set(
        "randomBytes",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, n: usize| {
            let mut bytes = vec![0u8; n];
            getrandom::fill(&mut bytes)
                .map_err(|e| rquickjs::Error::new_from_js_message("crypto", "randomBytes", &format!("getrandom failed: {e}")))?;
            let arr = rquickjs::Array::new(ctx)?;
            for (i, b) in bytes.iter().enumerate() {
                arr.set(i, *b as i32)?;
            }
            Ok::<_, rquickjs::Error>(arr)
        })?,
    )?;

    globals.set("crypto", crypto_obj)?;
    Ok(())
}

/// registers a stub that throws errors when crypto.* is called without the feature enabled
pub fn register_stub<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    let crypto_obj = Object::new(ctx.clone())?;

    let methods = ["md5", "sha256", "sha512", "hmacSha256", "hmacSha512", "randomUUID", "randomBytes"];
    for method in methods {
        crypto_obj.set(
            method,
            Function::new(ctx.clone(), || -> rquickjs::Result<()> {
                Err(rquickjs::Error::new_from_js_message("crypto", "method", "extension does not have 'crypto' feature enabled"))
            })?,
        )?;
    }

    globals.set("crypto", crypto_obj)?;
    Ok(())
}
