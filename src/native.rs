use super::value::Value;

#[derive(Debug, Clone)]
pub struct NativeFunc {
    pub name: String,
    arity: u8,
    func: fn(&[Value]) -> Result<Value, String>,
}

impl NativeFunc {
    pub fn call(&self, args: &[Value]) -> Result<Value, String> {
        if args.len() as u8 != self.arity {
            return Err(format!(
                "Expected {} arguments, but got {}.",
                self.arity,
                args.len()
            ));
        }

        (self.func)(args)
    }
}

/// Returns the current time in seconds since the start of the program
fn clock(_args: &[Value]) -> Result<Value, String> {
    let now = std::time::SystemTime::now();
    let since_unix_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Error: {:?}", e))?;
    let secs = (since_unix_epoch.as_millis() as f64) / 1000.0;

    Ok(Value::Number(secs))
}

pub fn get_native_funcs() -> Vec<NativeFunc> {
    vec![NativeFunc {
        name: "clock".to_string(),
        arity: 0,
        func: clock,
    }]
}
