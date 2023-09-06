/*
extension-version-watcher: rust program to check for updates in chrome extensions
Copyright (C) 2023  staticallyamazing

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::fmt::Debug;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Extension {
    pub name: String,
    pub display_name: String,
    pub id: String,
    pub url: Option<String>,
    pub generate_diff: bool,
}

impl Debug for Extension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        f.write_str(&self.display_name)?;
        f.write_str("\" [")?;
        f.write_str(&self.name)?;
        f.write_str("]")
    }
}

pub fn builtin_extensions() -> Vec<Extension> {
    vec![
        Extension {
            name: "classroom".into(),
            display_name: "Securly Classroom".into(),
            id: "jfbecfmiegcjddenjhlbhlikcbfmnafd".into(),
            url: Some("https://deviceconsole.securly.com/dist/chrome/n.xml".into()),
            generate_diff: true,
        },
        Extension {
            name: "chromebooks".into(),
            display_name: "Securly for Chromebooks [Old, Webstore]".into(),
            id: "iheobagjkfklnlikgihanlhcddjoihkg".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "chromebooks-new".into(),
            display_name: "Securly for Chromebooks [New]".into(),
            id: "joflmkccibkooplaeoinecjbmdebglab".into(),
            url: Some("https://extensions.securly.com/extensions.xml".into()),
            generate_diff: true,
        },
        Extension {
            name: "goguardian-stable".into(),
            display_name: "GoGuardian [Stable]".into(),
            id: "haldlgldplgnggkjaafhelgiaglafanh".into(),
            url: Some("https://ext.goguardian.com/stable.xml".into()),
            generate_diff: true,
        },
        Extension {
            name: "goguardian-alpha".into(),
            display_name: "GoGuardian [Alpha]".into(),
            id: "haldlgldplgnggkjaafhelgiaglafanh".into(),
            url: Some("https://ext.goguardian.com/alpha.xml".into()),
            generate_diff: true,
        },
        Extension {
            name: "blocksi".into(),
            display_name: "Blocksi".into(),
            id: "ghlpmldmjjhmdgmneoaibbegkjjbonbk".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "iboss".into(),
            display_name: "iBoss".into(),
            id: "kmffehbidlalibfeklaefnckpidbodff".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "fortiguard".into(),
            display_name: "Fortiguard".into(),
            id: "igbgpehnbmhgdgjbhkkpedommgmfbeao".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "cisco".into(),
            display_name: "Cisco".into(),
            id: "jcdhmojfecjfmbdpchihbeilohgnbdci".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "netref".into(),
            display_name: "NetRef".into(),
            id: "khfdeghnhlpdfeenmdofgcbilkngngcp".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "contentkeeper".into(),
            display_name: "ContentKeeper".into(),
            id: "jdogphakondfdmcanpapfahkdomaicfa".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "hapara".into(),
            display_name: "Hapara".into(),
            id: "kbohafcopfpigkjdimdcdgenlhkmhbnc".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "smoothwall".into(),
            display_name: "Smoothwall".into(),
            id: "jbldkhfglmgeihlcaeliadhipokhocnm".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "linewize".into(),
            display_name: "Linewize/Connect for Chrome".into(),
            id: "ddfbkhpmcdbciejenfcolaaiebnjcbfc".into(),
            url: None,
            generate_diff: true,
        },
        Extension {
            name: "lanschool".into(),
            display_name: "LANSchool".into(),
            id: "baleiojnjpgeojohhhfbichcodgljmnj".into(),
            url: None,
            generate_diff: true,
        },
    ]
}
