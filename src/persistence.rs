use std::sync::mpsc::Sender;

pub trait DbClient {
    // TODO: Modify this to take values other than strings.
    fn commit(&self, data: String) -> Result<(), String>;
}

pub struct DummyDbClient {

}

impl DbClient for DummyDbClient {
    fn commit(&self, _: String) -> Result<(), String> {
        return Ok(());
    }
}

pub struct InMemoryDbClient {
    // db_sender: Sender<String>
}

impl DbClient for InMemoryDbClient {
    fn commit(&self, data: String) -> Result<(), String> {
        // TODO: Check message was received correctly.
        // self.db_sender.send(data).expect("Receiver has been deallocated.");
        return Ok(());
    }
}

impl InMemoryDbClient {
    pub fn new() -> InMemoryDbClient {
        InMemoryDbClient {
            // db_sender
        }
    }
}