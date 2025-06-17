pub fn trace(msg: &str) {
    unsafe {
        ic0::debug_print(msg.as_ptr() as usize, msg.len() as usize);
    }
}

#[cfg(test)]
mod tests {}
