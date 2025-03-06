pub struct M6532Riot {
    ram: [u8; 128],
    swcha: u8,
    swacnt: u8,
    swchb: u8,
    swbcnt: u8,
    intim: u8,
    instat: u8,
}

pub struct M6532RiotConfig {}
