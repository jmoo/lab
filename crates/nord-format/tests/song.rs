//! In-memory `Song` construction / round-trip tests. These build values through
//! the API rather than reading specimen files, so they need no corpus and run in
//! the default (open) test suite.

use nord_format::common::bank::Item;
use nord_format::electro5;
use nord_format::error::Error;
use std::io::Cursor;

#[test]
fn test_ne5_read_write_new_song() -> Result<(), Error> {
    let mut song = electro5::Song::new(
        (0, 1).try_into()?,
        [
            (1, 2).try_into()?,
            (2, 3).try_into()?,
            (3, 4).try_into()?,
            (4, 5).try_into()?,
        ],
    );

    // Assert song was created with correct values
    assert_eq!(song.location(), (0, 1));
    assert_eq!(song.get(0), (1, 2));
    assert_eq!(song.get(1), (2, 3));
    assert_eq!(song.get(2), (3, 4));
    assert_eq!(song.get(3), (4, 5));

    // Read/Write song to result
    let mut write_result = Vec::new();
    song.write_to(&mut Cursor::new(&mut write_result)).unwrap();

    let result = electro5::Song::read_from(&mut Cursor::new(&mut write_result)).unwrap();

    // Assert those values are the same after writing and reading
    assert_eq!(song.location(), result.location());
    assert_eq!(song.get(0), result.get(0));
    assert_eq!(song.get(1), result.get(1));
    assert_eq!(song.get(2), result.get(2));
    assert_eq!(song.get(3), result.get(3));

    Ok(())
}

#[test]
fn test_ne5_update_song_program() -> Result<(), Error> {
    let mut song = electro5::Song::new(
        (0, 1).try_into()?,
        [
            (1, 2).try_into()?,
            (2, 3).try_into()?,
            (3, 4).try_into()?,
            (4, 5).try_into()?,
        ],
    );

    // Update program 1
    song.set(1, (5, 20).try_into()?);

    // Assert song was updated with correct values
    assert_eq!(song.location(), (0, 1));
    assert_eq!(song.get(0), (1, 2));
    assert_eq!(song.get(1), (5, 20));
    assert_eq!(song.get(2), (3, 4));
    assert_eq!(song.get(3), (4, 5));

    // Read/Write song to result
    let mut write_result = Vec::new();
    song.write_to(&mut Cursor::new(&mut write_result)).unwrap();

    let result = electro5::Song::read_from(&mut Cursor::new(&mut write_result)).unwrap();

    // Assert those values are the same after writing and reading
    assert_eq!(song.location(), result.location());
    assert_eq!(song.get(0), result.get(0));
    assert_eq!(song.get(1), result.get(1));
    assert_eq!(song.get(2), result.get(2));
    assert_eq!(song.get(3), result.get(3));

    Ok(())
}
