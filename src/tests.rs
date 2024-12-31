/* Copyright © 2024 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;

#[test]
fn basic_argument_parsing() {
    let _ = Args::parse_from(["binary", "some-folder"]);
}
