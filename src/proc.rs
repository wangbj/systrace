use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result};
use std::path::PathBuf;

use combine::error::ParseError;
use combine::parser::char::{char, digit, hex_digit, letter, spaces};
use combine::Parser;
use combine::{any, choice, count, many, many1, none_of, optional, Stream};

use libc;
use nix::unistd;
use nix::unistd::Pid;

#[derive(Clone)]
pub struct ProcMapsEntry {
    base: u64,
    size: u64,
    prot: i32,
    flags: i32,
    offset: u64,
    dev: i32,
    inode: u64,
    file: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
pub enum LinuxTaskState {
    Running,
    SleepInterruptible,
    SleepUninterruptible,
    Zombine,
    Stopped,
    Ptraced,
    Dead,
}

impl ProcMapsEntry {
    pub fn base(&self) -> u64 {
        self.base
    }
    pub fn size(&self) -> usize {
        self.size as usize
    }
    pub fn end(&self) -> u64 {
        self.base + self.size
    }
    pub fn filename(&self) -> Option<&PathBuf> {
        self.file.iter().next()
    }
}

pub fn pretty_show_maps(pid: Pid) -> String {
    let mut res = String::new();

    let ents = decode_proc_maps(pid).unwrap();
    for e in ents {
        let s = format!("{:?}\n", e);
        res.push_str(&s);
    }
    res
}

fn format_prot_flags(prot: i32, flags: i32) -> String {
    let mut res = String::new();
    if prot & libc::PROT_READ != 0 {
        res.push('r');
    } else {
        res.push('-');
    }
    if prot & libc::PROT_WRITE != 0 {
        res.push('w');
    } else {
        res.push('-');
    }
    if prot & libc::PROT_EXEC != 0 {
        res.push('x');
    } else {
        res.push('-');
    }
    if flags & libc::MAP_SHARED != 0 {
        res.push('s');
    } else if flags & libc::MAP_PRIVATE != 0 {
        res.push('p');
    } else {
        res.push('-');
    }
    res
}

impl std::fmt::Debug for ProcMapsEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut res = String::new();

        let fp = match &self.file {
            Some(path) => String::from(path.to_str().unwrap_or("")),
            None => String::from(""),
        };
        let s = format!(
            "{:x}-{:x} {} {:08x} {:02x}:{:02x} {}",
            self.base,
            self.base + self.size,
            &format_prot_flags(self.prot, self.flags),
            self.offset,
            self.dev.wrapping_shr(8),
            self.dev & 0xff,
            self.inode
        );
        res.push_str(&s);
        (0..=72 - s.len()).for_each(|_| res.push(' '));
        res.push_str(&fp);
        write!(f, "{}", res)
    }
}

fn hex_value<I>() -> impl Parser<Input = I, Output = u64>
where
    I: Stream<Item = char>,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    many1::<String, _>(hex_digit()).map(|s| u64::from_str_radix(&s, 16).unwrap_or(0))
}

fn dec_value<I>() -> impl Parser<Input = I, Output = u64>
where
    I: Stream<Item = char>,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    many1::<String, _>(hex_digit()).map(|s| s.parse::<u64>().unwrap_or(0))
}

fn dev<I>() -> impl Parser<Input = I, Output = i32>
where
    I: Stream<Item = char>,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    (
        spaces(),
        count::<String, _>(2, hex_digit()),
        char(':'),
        count::<String, _>(2, hex_digit()),
    )
        .map(|(_, major, _, minor)| {
            i32::from_str_radix(&major, 16).unwrap_or(0) * 256
                + i32::from_str_radix(&minor, 16).unwrap_or(0)
        })
}

fn prot<I>() -> impl Parser<Input = I, Output = (i32, i32)>
where
    I: Stream<Item = char>,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    (
        spaces(),
        choice([char('-'), char('r')]),
        choice([char('-'), char('w')]),
        choice([char('-'), char('x')]),
        choice([char('-'), char('s'), char('p')]),
    )
        .map(|(_, r, w, x, p)| {
            let mut prot: i32 = 0;
            let mut flags: i32 = 0;
            if r == 'r' {
                prot |= libc::PROT_READ;
            }
            if w == 'w' {
                prot |= libc::PROT_WRITE;
            }
            if x == 'x' {
                prot |= libc::PROT_EXEC;
            }
            if p == 'p' {
                flags |= libc::MAP_PRIVATE;
            } else if p == 's' {
                flags |= libc::MAP_SHARED;
            }
            (prot, flags)
        })
}

fn filepath<I>() -> impl Parser<Input = I, Output = Option<PathBuf>>
where
    I: Stream<Item = char>,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    (
        spaces(),
        optional(many1::<String, _>(none_of("\r\n".chars().into_iter()))),
    )
        .map(|(_, path)| path.map(|s| PathBuf::from(s)))
}

fn parse_proc_maps_entry(line: &str) -> Result<ProcMapsEntry> {
    match parser().easy_parse(line) {
        Ok((result, _)) => Ok(result),
        Err(parse_error) => Err(Error::new(
            ErrorKind::Other,
            format!("parse error: {}", parse_error),
        )),
    }
}

fn parser<I>() -> impl Parser<Input = I, Output = ProcMapsEntry>
where
    I: Stream<Item = char>,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    (
        hex_value(),
        char('-'),
        hex_value(),
        prot(),
        spaces(),
        hex_value(),
        dev(),
        spaces(),
        dec_value(),
        filepath(),
    )
        .map(
            |(from, _, to, (prot_val, flags_val), _, offset, devno, _, inode, path)| {
                ProcMapsEntry {
                    base: from,
                    size: to - from,
                    prot: prot_val,
                    flags: flags_val,
                    offset,
                    dev: devno,
                    inode,
                    file: path,
                }
            },
        )
}

pub fn decode_proc_maps(pid: Pid) -> Result<Vec<ProcMapsEntry>> {
    let filepath = PathBuf::from("/proc")
        .join(&format!("{}", pid))
        .join("maps");
    let mut file = File::open(filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let ents: Vec<Result<_>> = contents
        .lines()
        .map(|line| parse_proc_maps_entry(line))
        .collect();
    ents.into_iter().collect()
}

/// get task (`pid`) state by reading procfs
/// kernel 3.13+ is required, prior to 3.13, there're
/// more states, which we don't plan to support
pub fn proc_get_task_state(pid: Pid) -> Result<LinuxTaskState> {
    let stat = PathBuf::from("/proc")
        .join(&format!("{}", pid))
        .join("status");
    let err = Error::new(ErrorKind::Other,
                         format!("could not read {:?}", &stat));
    let contents = std::fs::read_to_string(stat)?;
    contents.lines().nth(2).and_then(|s| {
        match s.split_whitespace().nth(1) {
            Some("R") => Some(LinuxTaskState::Running),
            Some("S") => Some(LinuxTaskState::SleepInterruptible),
            Some("D") => Some(LinuxTaskState::SleepUninterruptible),
            Some("T") => Some(LinuxTaskState::Stopped),
            Some("t") => Some(LinuxTaskState::Ptraced),
            Some("X") => Some(LinuxTaskState::Dead),
            Some("Z") => Some(LinuxTaskState::Zombine),
            _         => None,
        }
    }).ok_or(err)
}

#[test]
fn can_decode_proc_self_maps() -> Result<()> {
    let my_pid = unistd::getpid();
    let decoded = decode_proc_maps(my_pid)?;
    assert!(decoded.len() > 0);
    Ok(())
}

#[test]
fn can_decode_proc_self_state() {
    let pid = unistd::getpid();
    let decoded = proc_get_task_state(pid);
    assert!(decoded.is_ok());
}
