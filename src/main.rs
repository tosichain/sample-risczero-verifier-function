use futures_util::TryStreamExt;
use ipfs_api::ApiError;
use ipfs_api::Error::Api;
use ipfs_api::TryFromUri;
use ipfs_api::{IpfsApi, IpfsClient};
use risc0_zkvm::receipt::Receipt;
use rs_merkle::MerkleTree;
use sha2::{Digest, Sha256};
use std::io::Cursor;
#[tokio::main]
async fn main() {
    let endpoint = "http://127.0.0.1:5001".to_string();

    let client = IpfsClient::from_str(&endpoint).unwrap();

    client.files_mkdir("/state", false).await;
    client.files_mkdir("/state/confirmed", false).await;

    match client.files_ls(Some("/input/image-id")).await {
        Ok(_) => {}
        Err(e) => match e {
            Api(api) => if api.message == "file does not exist".to_string() && api.code == 0{
                std::process::exit(0);
            },
            _ => {}
        },
    }

    match client.files_ls(Some("/input/receipts")).await {
        Ok(_) => {}
        Err(e) => match e {
            Api(api) => if api.message == "file does not exist".to_string() && api.code == 0{
                std::process::exit(0);
            },
            _ => {}
        },
    }

    for file in client
        .files_ls(Some("/input/receipts"))
        .await
        .unwrap()
        .entries
    {
        let serialized_receipt = client
            .files_read(&("/input/receipts/".to_string() + &file.name))
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await
            .unwrap();

        let receipt = serde_cbor::from_slice::<Receipt>(&serialized_receipt).unwrap();

        let serialized_image_id = client
            .files_read(&("/input/image-id/".to_string() + &file.name))
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await
            .unwrap();
        let image_id = serde_cbor::from_slice::<[u32; 8]>(&serialized_image_id).unwrap();

        let verification_result = receipt.verify(&image_id);
        match verification_result {
            Ok(_) => {
                let mut hash_vec = serialized_receipt;
                hash_vec.extend_from_slice(&serialized_image_id);
                let name_hash = Sha256::digest(hash_vec);

                client
                    .files_write(
                        &("/state/confirmed/".to_string() + &hex::encode(name_hash)),
                        true,
                        true,
                        Cursor::new(""),
                    )
                    .await
                    .unwrap();
            }
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }
    let merkle_tree_leaves: Vec<[u8; 32]> = client
        .files_ls(Some("/state/confirmed"))
        .await
        .unwrap()
        .entries
        .into_iter()
        .map(|file_name| {
            let decoded = hex::decode(file_name.name).unwrap();
            let mut result: [u8; 32] = [0; 32];
            result.copy_from_slice(&decoded[..32]);
            result
        })
        .collect();
    let merkle_tree = MerkleTree::<rs_merkle::algorithms::Sha256>::from_leaves(&merkle_tree_leaves);
    let merkle_root = merkle_tree
        .root()
        .ok_or("couldn't get the merkle root")
        .unwrap();

    let data = Cursor::new(merkle_root);

    client
        .files_write("/state/output.file", true, true, data)
        .await
        .unwrap();
}
