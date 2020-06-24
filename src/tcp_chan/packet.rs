pub enum Packet<T> {
    HandshakeRequest,
    HandshakeResponse,
    Msg(T)
}