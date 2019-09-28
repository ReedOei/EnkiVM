use num_bigint::BigInt;

#[derive(PartialEq, Clone, Debug)]
pub enum Const {
    IntConst(BigInt)
}

impl std::fmt::Display for Const {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Const::IntConst(i) => write!(f, "{}", i)
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum StackItem {
    Variable(String),
    ConstItem(Const)
}
