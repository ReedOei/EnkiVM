use num_bigint::BigInt;

#[derive(PartialEq, Clone, Debug)]
pub enum Value {
    IntValue(BigInt),
    StringValue(String),
    Functor(String, Vec<StackItem>)
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::IntValue(i) => write!(f, "{}", i),
            Value::StringValue(s) => write!(f, "{}", s),
            Value::Functor(name, args) => {
                let str_args: Vec<String> = args.iter().map(|arg| format!("{}", arg)).collect();
                write!(f, "{}({})", name, str_args.join(", "))
            }
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum StackItem {
    Variable(String),
    Value(Value)
}

impl std::fmt::Display for StackItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StackItem::Variable(s) => write!(f, "{}", s),
            StackItem::Value(c) => write!(f, "{}", c)
        }
    }
}
