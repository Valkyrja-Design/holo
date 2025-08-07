#[derive(Clone, Copy, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
}
