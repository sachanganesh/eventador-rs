pub enum NodeFlags {
    ReadOnly(usize),
    WriteOnly(usize),
    ReadWrite(usize)
}

pub struct HandshakeRequest {
    flags: Vec<NodeFlags>,
    node_name: Option<String>,
}

pub struct HandshakeResponse {
    suggested_name: Option<String>,
    challenge: String
}
