use ninep::{
    Result,
    fs::{FileMeta, IoUnit, Mode, Perm, Stat},
    server::{ClientId, ReadOutcome, Serve9p, Server},
};
use std::{
    sync::{Arc, RwLock, mpsc::channel},
    thread::{sleep, spawn},
    time::{Duration, SystemTime},
};

struct BigFs {}

//impl Serve9p for BigFs {}
