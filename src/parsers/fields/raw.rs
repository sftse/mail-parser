/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use crate::{parsers::message::MessageStream, HeaderValue};

pub fn parse_raw<'x>(stream: &mut MessageStream<'x>) -> HeaderValue<'x> {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;

    let mut iter = stream.data[stream.pos..].iter();

    while let Some(ch) = iter.next() {
        stream.pos += 1;
        match ch {
            b'\n' => match stream.data.get(stream.pos) {
                Some(b' ' | b'\t') => {
                    iter.next();
                    stream.pos += 1;
                    continue;
                }
                _ => {
                    return if token_start > 0 {
                        HeaderValue::Text(String::from_utf8_lossy(
                            &stream.data[token_start - 1..token_end],
                        ))
                    } else {
                        HeaderValue::Empty
                    };
                }
            },
            b' ' | b'\t' | b'\r' => continue,
            _ => (),
        }

        if token_start == 0 {
            token_start = stream.pos;
        }

        token_end = stream.pos;
    }

    HeaderValue::Empty
}

pub fn parse_and_ignore(stream: &mut MessageStream) {
    let mut iter = stream.data[stream.pos..].iter();

    while let Some(ch) = iter.next() {
        stream.pos += 1;

        if ch == &b'\n' {
            match stream.data.get(stream.pos) {
                Some(b' ' | b'\t') => {
                    iter.next();
                    stream.pos += 1;
                    continue;
                }
                _ => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parsers::fields::raw::parse_raw;
    use crate::parsers::message::MessageStream;
    use crate::{HeaderName, Message, RfcHeader};

    #[test]
    fn parse_raw_text() {
        let inputs = [
            ("Saying Hello\nMessage-Id", "Saying Hello"),
            ("Re: Saying Hello\r\n \r\nFrom:", "Re: Saying Hello"),
            (
                concat!(
                    " from x.y.test\n      by example.net\n      via TCP\n",
                    "      with ESMTP\n      id ABC12345\n      ",
                    "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600\n"
                ),
                concat!(
                    "from x.y.test\n      by example.net\n      via TCP\n",
                    "      with ESMTP\n      id ABC12345\n      ",
                    "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600"
                ),
            ),
        ];

        for input in inputs {
            let str = input.0.to_string();
            assert_eq!(
                parse_raw(&mut MessageStream::new(str.as_bytes())).unwrap_text(),
                input.1,
                "Failed for '{:?}'",
                input.0
            );
        }
    }

    #[test]
    fn ordered_raw_headers() {
        let input = br#"From: Art Vandelay <art@vandelay.com>
To: jane@example.com
Date: Sat, 20 Nov 2021 14:22:01 -0800
Subject: Why not both importing AND exporting? =?utf-8?b?4pi6?=
Content-Type: multipart/mixed; boundary="festivus";

Here's a message body.
"#;
        let message = Message::parse(input).unwrap();
        let mut iter = message.get_raw_headers();
        assert_eq!(
            iter.next().unwrap(),
            (
                &HeaderName::Rfc(RfcHeader::From),
                " Art Vandelay <art@vandelay.com>\n".into()
            )
        );
        assert_eq!(
            iter.next().unwrap(),
            (
                &HeaderName::Rfc(RfcHeader::To),
                " jane@example.com\n".into()
            )
        );
        assert_eq!(
            iter.next().unwrap(),
            (
                &HeaderName::Rfc(RfcHeader::Date),
                " Sat, 20 Nov 2021 14:22:01 -0800\n".into()
            )
        );
        assert_eq!(
            iter.next().unwrap(),
            (
                &HeaderName::Rfc(RfcHeader::Subject),
                " Why not both importing AND exporting? =?utf-8?b?4pi6?=\n".into()
            )
        );
        assert_eq!(
            iter.next().unwrap(),
            (
                &HeaderName::Rfc(RfcHeader::ContentType),
                " multipart/mixed; boundary=\"festivus\";\n".into()
            )
        );
    }
}
