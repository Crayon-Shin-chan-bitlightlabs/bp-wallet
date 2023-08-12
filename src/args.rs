// Modern, minimalistic & standard-compliant cold wallet library.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2020-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2020-2023 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2020-2023 Dr Maxim Orlovsky. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::Debug;
use std::path::PathBuf;

use bp::{DeriveSpk, Keychain};
use bp_rt::Runtime;
use clap::Subcommand;
use strict_encoding::Ident;

use crate::opts::{DescrStdOpts, DescriptorOpts};
use crate::{BoostrapError, Config, GeneralOpts, ResolverOpt, WalletOpts};

/// Command-line arguments
#[derive(Parser)]
#[derive(Clone, Eq, PartialEq, Debug)]
#[command(author, version, about)]
pub struct Args<C: Clone + Eq + Debug + Subcommand, O: DescriptorOpts = DescrStdOpts> {
    /// Set verbosity level.
    ///
    /// Can be used multiple times to increase verbosity.
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(flatten)]
    pub wallet: WalletOpts<O>,

    #[command(flatten)]
    pub resolver: ResolverOpt,

    #[command(flatten)]
    pub general: GeneralOpts,

    /// Command to execute.
    #[clap(subcommand)]
    pub command: C,
}

pub trait Exec {
    type Error: std::error::Error;
    const CONF_FILE_NAME: &'static str;

    fn exec<C: Keychain>(self, config: Config) -> Result<(), Self::Error>
    where for<'de> C: serde::Serialize + serde::Deserialize<'de>;
}

impl<C: Clone + Eq + Debug + Subcommand, O: DescriptorOpts> Args<C, O>
where Self: Exec
{
    pub fn conf_path(&self) -> PathBuf {
        let mut conf_path = self.general.base_dir();
        conf_path.push("bp.toml");
        conf_path
    }
}

impl<C: Clone + Eq + Debug + Subcommand, O: DescriptorOpts> Args<C, O> {
    pub fn process(&mut self) { self.general.process(); }

    pub fn bp_runtime<D: DeriveSpk, K: Keychain>(
        &self,
        conf: &Config,
    ) -> Result<Runtime<D, K>, BoostrapError>
    where
        for<'de> D: From<O::Descr> + serde::Serialize + serde::Deserialize<'de>,
        for<'de> K: serde::Serialize + serde::Deserialize<'de>,
    {
        eprint!("Loading descriptor");
        let mut runtime: Runtime<D, K> = if let Some(d) = self.wallet.descriptor_opts.descriptor() {
            eprint!(" from command-line argument ... ");
            Runtime::new(d.into(), self.general.chain)
        } else if let Some(wallet_path) = self.wallet.wallet_path.clone() {
            eprint!(" from specified wallet directory ... ");
            Runtime::load(wallet_path)?
        } else {
            let wallet_name = self
                .wallet
                .name
                .as_ref()
                .map(Ident::to_string)
                .unwrap_or(conf.default_wallet.clone());
            eprint!(" from wallet {wallet_name} ... ");
            Runtime::load(self.general.wallet_dir(wallet_name))?
        };
        eprintln!("success");

        if self.resolver.sync || self.wallet.descriptor_opts.is_some() {
            eprint!("Syncing ...");
            let indexer = esplora::Builder::new(&self.resolver.esplora).build_blocking()?;
            if let Err(errors) = runtime.sync(&indexer) {
                eprintln!(" partial, some requests has failed:");
                for err in errors {
                    eprintln!("- {err}");
                }
            } else {
                eprintln!(" success");
            }
        }

        Ok(runtime)
    }
}
