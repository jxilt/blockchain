pub trait DbClient {
    // TODO: Modify this to take values other than strings.
    fn commit(&self, data: String) -> Result<(), String>;
}

/// A no-op database client used for testing.
pub struct DummyDbClient {

}

impl DbClient for DummyDbClient {
    fn commit(&self, _: String) -> Result<(), String> {
        return Ok(());
    }
}

pub struct InMemoryDbClient {}

impl InMemoryDbClient {
    pub fn new() -> InMemoryDbClient {
        InMemoryDbClient {

        }
    }
}

impl DbClient for InMemoryDbClient {
    fn commit(&self, _data: String) -> Result<(), String> {
        // TODO: Store data.
        return Ok(());
    }
}