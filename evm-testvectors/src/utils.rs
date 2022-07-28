use std::str::FromStr;

use anyhow::{bail, Result};
use eth_types::{evm_types::OpcodeId, GethExecTrace, U256};
use prettytable::Table;
use zkevm_circuits::test_util::{get_fixed_table, BytecodeTestConfig, FixedTableConfig};

const OPCODES_NEED_FULL_FIXED_TABLE: [OpcodeId; 3] = [OpcodeId::AND, OpcodeId::OR, OpcodeId::XOR];

// see https://github.com/appliedzkp/zkevm-circuits/issues/477
pub const OPCODES_UNIMPLEMENTED: [OpcodeId; 20] = [
    OpcodeId::EXP,
    OpcodeId::SHL,
    OpcodeId::SHR,
    OpcodeId::SAR,
    OpcodeId::RETURN,
    OpcodeId::REVERT,
    OpcodeId::SHA3,
    OpcodeId::ADDRESS,
    OpcodeId::BALANCE,
    OpcodeId::EXTCODESIZE,
    OpcodeId::EXTCODECOPY,
    OpcodeId::RETURNDATASIZE,
    OpcodeId::RETURNDATACOPY,
    OpcodeId::BLOCKHASH,
    OpcodeId::CREATE,
    OpcodeId::CREATE2,
    OpcodeId::CALLCODE,
    OpcodeId::DELEGATECALL,
    OpcodeId::STATICCALL,
    OpcodeId::SELFDESTRUCT,
];

#[derive(Debug, PartialEq, PartialOrd)]
pub enum MainnetFork {
    Merge = 14,
    GrayGlacier = 13,
    ArrowGlacier = 12,
    Altair = 11,
    London = 10,
    Berlin = 9,
    MuirGlacier = 8,
    Istanbul = 7,
    Constantinople = 6,
    Byzantium = 5,
    SpuriousDragon = 4,
    TangerineWhistle = 3,
    Homestead = 2,
    Frontier = 1,
}

pub const TEST_FORK: MainnetFork = MainnetFork::Merge;

impl FromStr for MainnetFork {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Merge" => Self::Merge,
            "Gray Glacier" => Self::GrayGlacier,
            "Arrow Glacier" => Self::ArrowGlacier,
            "Altair" => Self::Altair,
            "London" => Self::London,
            "Berlin" => Self::Berlin,
            "Muir Glacier" => Self::MuirGlacier,
            "Istanbul" => Self::Istanbul,
            "Constantinople" => Self::Constantinople,
            "Byzantium" => Self::Byzantium,
            "Spurious Dragon" => Self::SpuriousDragon,
            "TangeringWhistle" => Self::TangerineWhistle,
            "Homestead" => Self::Homestead,
            "Frontier" => Self::Frontier,
            _ => bail!(format!("Unknown network '{}'", s)),
        })
    }
}

impl MainnetFork {
    pub fn in_network_range(expect: &[String]) -> Result<bool, anyhow::Error> {
        let in_network = if expect.is_empty() {
            true
        } else {
            let mut in_network = false;
            for network in expect {
                if let Some(network) = network.strip_prefix(">=") {
                    if crate::utils::TEST_FORK >= crate::utils::MainnetFork::from_str(network)? {
                        in_network = true;
                    }
                } else {
                    if crate::utils::TEST_FORK == crate::utils::MainnetFork::from_str(&network)? {
                        in_network = true;
                    }
                }
            }
            in_network
        };

        Ok(in_network)
    }
}

pub fn config_bytecode_test_config<OPS: Iterator<Item = OpcodeId>>(
    cfg: &mut BytecodeTestConfig,
    mut ops: OPS,
) {
    let needs_complete_fixed_table = ops.any(|op| OPCODES_NEED_FULL_FIXED_TABLE.contains(&op));

    if needs_complete_fixed_table {
        cfg.evm_circuit_lookup_tags = get_fixed_table(FixedTableConfig::Complete);
    }
}

pub fn print_trace(trace: GethExecTrace) -> Result<()> {
    fn u256_to_str(u: &U256) -> String {
        if u.leading_zeros() < 26 {
            format!("{:x}", u)
        } else {
            u.to_string()
        }
    }
    fn kv(storage: std::collections::HashMap<U256, U256>) -> Vec<String> {
        let mut keys: Vec<_> = storage.keys().collect();
        keys.sort();
        keys.iter()
            .map(|k| format!("{}: {}", u256_to_str(k), u256_to_str(&storage[k])))
            .collect()
    }
    fn split(strs: Vec<String>, len: usize) -> String {
        let mut out = String::new();
        let mut current_len = 0;
        let mut it = strs.iter();
        let mut current = it.next();

        while let Some(v) = current {
            let mut count = 1usize;
            current = it.next();
            while current == Some(v) {
                count += 1;
                current = it.next();
            }

            let item = if count == 1 {
                v.to_string()
            } else {
                format!("{}[{}]", v, count)
            };

            if current_len > len {
                current_len = 0;
                out.push('\n');
            } else if current_len > 0 {
                out.push_str(", ");
            }
            out.push_str(&item);
            current_len += item.len();
        }
        out
    }

    let mut table = Table::new();
    table.add_row(row![
        "PC", "OP", "GAS", "GAS_COST", "DEPTH", "ERR", "STACK", "MEMORY", "STORAGE"
    ]);
    for step in trace.struct_logs {
        table.add_row(row![
            format!("{}", step.pc.0),
            format!("{:?}", step.op),
            format!("{}", step.gas.0),
            format!("{}", step.gas_cost.0),
            format!("{}", step.depth),
            step.error.unwrap_or_else(|| "".to_string()),
            split(step.stack.0.iter().map(u256_to_str).collect(), 30),
            split(step.memory.0.iter().map(ToString::to_string).collect(), 30),
            split(kv(step.storage.0), 30)
        ]);
    }

    println!("FAILED: {:?}", trace.failed);
    println!("GAS: {:?}", trace.gas);
    table.printstd();

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn networks() { 
        assert!(MainnetFork::in_network_range(&[String::from(">=Istanbul")]).expect("can parse network"));
    }

}