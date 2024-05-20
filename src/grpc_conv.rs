use std::str::FromStr;

use massa_serialization::Serializer;
use massa_hash::Hash;
use massa_models::address::Address;
use massa_models::amount::{Amount, AMOUNT_DECIMAL_SCALE};
use massa_models::block::{Block, BlockSerializer, SecureShareBlock};
use massa_models::block_header::{BlockHeader, BlockHeaderSerializer, SecuredHeader};
use massa_models::block_id::BlockId;
use massa_models::config::CHAINID;
use massa_models::endorsement::{Endorsement, EndorsementId, EndorsementSerializer, SecureShareEndorsement};
use massa_models::operation::{Operation, OperationId, OperationSerializer, OperationType, SecureShareOperation};
use massa_models::secure_share::{Id, SecureShare, SecureShareContent};
use massa_signature::{PublicKey, Signature};
use massa_proto_rs::massa::model::v1::{self as grpc_model};


pub fn secure_share_block_from_filled_block(f_b: grpc_model::FilledBlock) -> SecureShareBlock {
    let content = Block {
        header: secure_header_from_signed_block_header(f_b.header.clone().unwrap()),
        operations: secure_shared_operations_from_filled_operation_entries(&f_b.operations)
            .iter()
            .map(|op| op.id)
            .collect(),
    };
    let block_serializer = BlockSerializer::new();
    let mut serialized_data = Vec::new();
    block_serializer
        .serialize(&content, &mut serialized_data)
        .unwrap();

    let content_creator_pub_key =
        PublicKey::from_str(&f_b.header.as_ref().unwrap().content_creator_pub_key).unwrap();

    let hash = content.compute_hash(&serialized_data, &content_creator_pub_key, *CHAINID);

    SecureShareBlock {
        content,
        serialized_data,
        signature: Signature::from_str(&f_b.header.as_ref().unwrap().signature).unwrap(),
        content_creator_pub_key,
        content_creator_address: address_from_str(&f_b.header.unwrap().content_creator_address),
        id: BlockId::new(hash),
    }
}

fn secure_header_from_signed_block_header(s_bh: grpc_model::SignedBlockHeader) -> SecuredHeader {
    let content = block_header_from_grpc_block_header(s_bh.content.unwrap());

    let header_serializer = BlockHeaderSerializer::new();
    let mut serialized_data = Vec::new();
    header_serializer
        .serialize(&content, &mut serialized_data)
        .unwrap();

    let content_creator_pub_key = PublicKey::from_str(&s_bh.content_creator_pub_key).unwrap();
    let hash = content.compute_hash(&serialized_data, &content_creator_pub_key, *CHAINID);
    SecuredHeader {
        content,
        serialized_data,
        signature: Signature::from_str(&s_bh.signature).unwrap(),
        content_creator_pub_key,
        content_creator_address: address_from_str(&s_bh.content_creator_address),
        id: BlockId::new(hash),
    }
}
fn block_header_from_grpc_block_header(block_header: grpc_model::BlockHeader) -> BlockHeader {
    BlockHeader {
        current_version: block_header.current_version,
        announced_version: block_header.announced_version,
        slot: block_header.slot.unwrap().into(),
        parents: block_header
            .parents
            .iter()
            .map(|bid| BlockId::from_str(bid).unwrap())
            .collect(),
        operation_merkle_root: Hash::from_str(&block_header.operations_hash).unwrap(),
        endorsements: endorsements_form_signed_endorsements(&block_header.endorsements),
        // FIXME can't create denunciation from serialized data
        denunciations: Vec::new(),
    }
}

// fn denunciation_from_grpc_denunciation(g_denun: grpc_model::Denunciation) -> Denunciation {
//     match g_denun.entry.unwrap() {
//         grpc_model::denunciation::Entry::BlockHeader(b) => {
//             Denunciation::BlockHeader(BlockHeaderDenunciation {
//                 public_key: PublicKey::from_str(b.public_key).unwrap(),
//                 slot: b.slot,
//                 hash_1: b.hash_1,
//                 hash_2: b.hash_2,
//                 signature_1: b.signature_1,
//                 signature_2: b.signature_2,
//             })
//         }
//         grpc_model::denunciation::Entry::Endorsement(e) => {
//             Denunciation::Endorsement(EndorsementDenunciation {
//                 public_key: todo!(),
//                 slot: todo!(),
//                 index: todo!(),
//                 hash_1: todo!(),
//                 hash_2: todo!(),
//                 signature_1: todo!(),
//                 signature_2: todo!(),
//             })
//         }
//     }
// }

fn endorsements_form_signed_endorsements(
    endorsements: &[grpc_model::SignedEndorsement],
) -> Vec<SecureShare<Endorsement, EndorsementId>> {
    endorsements
        .into_iter()
        .map(|s_endo| secure_share_endorsement_from_signed_endorsement(s_endo.to_owned()))
        .collect()
}

fn secure_share_endorsement_from_signed_endorsement(
    s_endo: grpc_model::SignedEndorsement,
) -> SecureShareEndorsement {
    let content = s_endo.content.unwrap();

    let content = Endorsement {
        slot: content.slot.unwrap().into(),
        index: content.index,
        endorsed_block: BlockId::from_str(&content.endorsed_block).unwrap()
    };

    let endo_serializer = EndorsementSerializer::new();
    let mut serialized_data = Vec::new();
    endo_serializer
        .serialize(&content, &mut serialized_data)
        .expect("Failed to serialize endorsement");

    let content_creator_pub_key = PublicKey::from_str(&s_endo.content_creator_pub_key).unwrap();
    let hash = content.compute_hash(&serialized_data, &content_creator_pub_key, *CHAINID);

    SecureShareEndorsement {
        content,
        serialized_data,
        signature: Signature::from_str(&s_endo.signature).unwrap(),
        content_creator_pub_key,
        content_creator_address: address_from_str(&s_endo.content_creator_address),
        id: EndorsementId::new(hash),
    }
}

pub fn secure_shared_operations_from_filled_operation_entries(
    operations: &[grpc_model::FilledOperationEntry],
) -> Vec<SecureShare<Operation, OperationId>> {
    operations
        .into_iter()
        .map(
            |grpc_model::FilledOperationEntry {
                 operation_id: _,
                 operation,
             }| {
                match operation {
                    Some(op) => {
                        secure_share_operation_from_signed_operation(op.clone())
                    }
                    None => {
                        // TODO: panic if op is None?
                        todo!()
                    }
                }
            },
        )
        .collect()
}

fn secure_share_operation_from_signed_operation(
    s_op: grpc_model::SignedOperation,
) -> SecureShareOperation {
    let content: grpc_model::Operation = s_op.content.expect("Missing operation content");
    let fee = content.fee.expect("Missing operation fee");

    let fee = amount_from_native_amount(fee);

    let op: OperationType = match content.op {
        Some(op_type) => match op_type.r#type {
            Some(op_type) => operation_type_from_op_type(op_type),

            None => panic!("Missing operation type"),
        },
        None => panic!("Missing operation details"),
    };

    let op: Operation = Operation {
        fee,
        expire_period: content.expire_period,
        op,
    };

    let op_serializer = OperationSerializer::new();
    let mut serialized_data = Vec::new();
    op_serializer
        .serialize(&op, &mut serialized_data)
        .expect("Failed to serialize operation");

    let content_creator_pub_key = PublicKey::from_str(&s_op.content_creator_pub_key).unwrap();

    let hash = op.compute_hash(&serialized_data, &content_creator_pub_key, *CHAINID);

    SecureShareOperation {
        content: op,
        serialized_data,
        signature: Signature::from_str(&s_op.signature).unwrap(),
        content_creator_pub_key,
        content_creator_address: address_from_str(&s_op.content_creator_address),
        id: OperationId::new(hash),
    }
}

fn operation_type_from_op_type(op_type: grpc_model::operation_type::Type) -> OperationType {
    match op_type {
        grpc_model::operation_type::Type::Transaction(transaction) => OperationType::Transaction {
            recipient_address: address_from_str(&transaction.recipient_address),
            amount: amount_from_native_amount(transaction.amount.unwrap()),
        },
        grpc_model::operation_type::Type::RollBuy(roll_buy) => OperationType::RollBuy {
            roll_count: roll_buy.roll_count,
        },
        grpc_model::operation_type::Type::RollSell(roll_sell) => OperationType::RollSell {
            roll_count: roll_sell.roll_count,
        },
        grpc_model::operation_type::Type::ExecutSc(execute_sc) => OperationType::ExecuteSC {
            data: execute_sc.data,
            max_gas: execute_sc.max_gas,
            max_coins: amount_from_raw(execute_sc.max_coins),
            datastore: execute_sc
                .datastore
                .into_iter()
                .map(|bytes_map| (bytes_map.key, bytes_map.value))
                .collect(),
        },
        grpc_model::operation_type::Type::CallSc(call_sc) => OperationType::CallSC {
            target_addr: address_from_str(&call_sc.target_address),
            target_func: call_sc.target_function,
            param: call_sc.parameter,
            max_gas: call_sc.max_gas,
            coins: amount_from_native_amount(call_sc.coins.unwrap()),
        },
    }
}

pub fn address_from_str(addr: &str) -> Address {
    Address::from_str(addr).unwrap_or_else(|_| panic!("Failed to parse Address from string"))
}

fn amount_from_native_amount(na: grpc_model::NativeAmount) -> Amount {
    Amount::from_mantissa_scale(na.mantissa, na.scale).unwrap_or_else(|_| {
        panic!(
            "Failed to convent mantissa {} and scale {} to Amount",
            na.mantissa, na.scale
        )
    })
}

fn amount_from_raw(raw: u64) -> Amount {
    amount_from_native_amount(grpc_model::NativeAmount {
        mantissa: raw,
        scale: AMOUNT_DECIMAL_SCALE,
    })
}