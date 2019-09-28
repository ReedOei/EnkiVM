#[derive(Debug)]
pub struct Err {
    msg: String
}

impl Err {
    pub fn new(msg: String) -> Err {
        Err {
            msg: msg
        }
    }

    pub fn err_res<T>(msg: String) -> Result<T, Err> {
        Err(Err::new(msg))
    }

    pub fn msg_clone(&self) -> String {
        self.msg.clone()
    }
}
