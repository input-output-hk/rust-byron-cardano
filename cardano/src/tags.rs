pub enum SigningTag {
    Tx = 0x01,
    RedeemTx = 0x02,
    VssCert = 0x03,
    USProposal = 0x04,
    Commitment = 0x05,
    USVote = 0x06,
    MainBlock = 0x07,
    MainBlockLight = 0x08,
    MainBlockHeavy = 0x09,
    ProxySK = 0x0a,
}
