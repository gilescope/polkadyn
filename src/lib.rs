use frame_metadata::RuntimeMetadata;
// use scale_info::PortableRegistry;
use scale_value::Value;
mod types_that_should_be_defined_somewhere_else;
// use scale_info::PortableType;
use parity_scale_codec::Compact;
use parity_scale_codec::Decode;
use types_that_should_be_defined_somewhere_else::Phase;

pub fn decode_events(
    metadata: &frame_metadata::RuntimeMetadataPrefixed,
    scale_encoded_data: &[u8],
) -> Result<Vec<(Phase, Value<()>)>, ()> {
    if let RuntimeMetadata::V14(metadata) = &metadata.1 {
        let mut event_type = None;
        for r in metadata.types.types() {
            if r.ty().path().segments() == &["polkadot_runtime", "Event"] {
                event_type = Some(r);
                break;
            }
        }
        let event_type = event_type.unwrap();
        let cursor = &mut &*scale_encoded_data;
        let mut num_events = <Compact<u32>>::decode(cursor).unwrap_or(Compact(0)).0;

        let mut results = Vec::with_capacity(num_events as usize);
        while num_events > 0 {
            let phase = Phase::decode(cursor).unwrap();
            let new_value =
                scale_value::scale::decode_as_type(cursor, event_type.id(), &metadata.types)
                    .unwrap();
            num_events -= 1;
            results.push((phase, new_value.remove_context()));
            let _topics = Vec::<[u8; 32]>::decode(cursor).unwrap(); //TODO don't hardcode hash size
        }

        Ok(results)
    } else {
        Err(())
    }
}

pub fn convert_json_block_response(
    json_response: &serde_json::value::Value,
) -> Result<(u32, Vec<Vec<u8>>), ()> {
    if let Some(serde_json::value::Value::Object(map)) = json_response.get("block") {
        let mut block_number: u32 = 0;
        let mut extrinsics = vec![];

        if let Some(serde_json::value::Value::Object(m)) = map.get("header") {
            if let Some(serde_json::value::Value::String(num_original)) = m.get("number") {
                block_number =
                    u32::from_str_radix(num_original.trim_start_matches("0x"), 16).unwrap();
            }
        }
        if let Some(serde_json::value::Value::Array(exs)) = map.get("extrinsics") {
            for ex in exs {
                if let serde_json::value::Value::String(val) = ex {
                    extrinsics.push(hex::decode(val.trim_start_matches("0x")).unwrap());
                } else {
                    panic!()
                }
            }
        }
        return Ok((block_number, extrinsics));
    }
    Err(())
}

pub fn decode_extrinsic(
    meta: &frame_metadata::RuntimeMetadataPrefixed,
    mut scale_encoded_data: &[u8],
) -> Result<Value<()>, ()> {
    if let RuntimeMetadata::V14(metadata) = &meta.1 {
        let mut extrinsic_type = None;
        for r in metadata.types.types() {
            if r.ty().path().segments() == &["polkadot_runtime", "Call"] {
                extrinsic_type = Some(r);
                break;
            }
        }

        let _size = <Compact<u32>>::decode(&mut scale_encoded_data)
            .unwrap_or(Compact(0))
            .0;

        let is_signed = scale_encoded_data[0] & 0b1000_0000 != 0;
        let version = scale_encoded_data[0] & 0b0111_1111;
        scale_encoded_data = &scale_encoded_data[1..];

        // We only know how to decode V4 extrinsics at the moment
        if version != 4 {
            // eprintln!(
            //     "not v4 v{} {} {}",
            //     version,
            //     is_signed,
            //     hex::encode(scale_encoded_data)
            // );
            return Err(());
        }

        // If the extrinsic is signed, decode the signature next.
        let _signature: Option<()> = match is_signed {
            true => {
                // skip_decode(meta, &["polkadot_runtime", "Call"], scale_encoded_data);
                let _address = <[u8; 32]>::decode(&mut scale_encoded_data); // TODO assumed 32 len. Can we figure out this from the metadata?
                let _sig = <[u8; 65 + 1]>::decode(&mut scale_encoded_data); // 1 byte for the discriminant.
                let _additional_and_extra_params = <[u8; 4]>::decode(&mut scale_encoded_data);
                Some(())
            }
            false => None,
        };

        // let cursor = &mut &*scale_encoded_data;

        let new_value = scale_value::scale::decode_as_type(
            &mut &*scale_encoded_data,
            extrinsic_type.unwrap().id(),
            &metadata.types,
        )
        .unwrap();
        Ok(new_value.remove_context())
    } else {
        Err(())
    }
}

pub fn potluck_decode(
    metadata: &frame_metadata::RuntimeMetadataPrefixed,
    scale_encoded_data: &[u8],
) {
    let mut clone = scale_encoded_data.clone();
    if let RuntimeMetadata::V14(metadata) = &metadata.1 {
        for r in metadata.types.types() {
            if scale_value::scale::decode_as_type(&mut clone, r.id(), &metadata.types).is_ok() {
                println!("can decode to {:?}", r.ty().path().segments())
            }
        }

        println!("fin potluck.....");
    }
}

pub fn skip_decode(
    metadata: &frame_metadata::RuntimeMetadataPrefixed,
    path: &[&str],
    scale_encoded_data: &[u8],
) {
    if let RuntimeMetadata::V14(metadata) = &metadata.1 {
        for r in metadata.types.types() {
            if r.ty().path().segments() == path {
                for i in 0..scale_encoded_data.len() {
                    let mut sub = &scale_encoded_data[i..];
                    if scale_value::scale::decode_as_type(&mut sub, r.id(), &metadata.types).is_ok()
                    {
                        println!("can decode at {}", i);
                    }
                }
            }
        }

        println!("fin skips.....");
    }
}

// PENDING PortableType being made pub.
// fn find_type<'reg>(registry: &'reg PortableRegistry, needle_path: &[&str]) -> Option<&'reg PortableType> {
//   let found = None;
//   for r in registry.types() {
//      if r.ty().path().segments() == needle_path {
//         found = Some(r);
//         break;
//      }
//   }
//   found
// }

#[cfg(test)]
mod tests {
    use super::*;

    use frame_metadata::RuntimeMetadata;
    use parity_scale_codec::Decode;
    use polkapipe::Backend;
    #[test]
    fn can_decode_extrinsics() {
        async_std::task::block_on(test_extrinsics());
    }

    async fn test_extrinsics() {
        // let hex_block_hash = "e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16";
        let hex_block_hash = "7b735190150afedb7e3ec930b1aba4fa828764fedf308281bf9666ffde2b62bd";
        let block_hash = hex::decode(hex_block_hash).unwrap();

        let client = polkapipe::ws::Backend::new_ws2("wss://rpc.polkadot.io")
            .await
            .unwrap();

        let metadata = client.query_metadata(Some(&block_hash[..])).await.unwrap();
        let meta =
            frame_metadata::RuntimeMetadataPrefixed::decode(&mut metadata.as_slice()).unwrap();
        assert!(matches!(meta.1, RuntimeMetadata::V14(_)));

        // let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
        // let key = hex::decode(events_key).unwrap();

        let block_json = client.query_block(hex_block_hash).await.unwrap();

        let (block_number, extrinsics) = convert_json_block_response(&block_json).unwrap();

        println!("number! {} {}", block_number, extrinsics.len());
        for (i, ex) in extrinsics.iter().enumerate() {
            let res = decode_extrinsic(&meta, &ex[..]);
            println!("just finished decoding {} res was {:?}", i, res.is_ok());
        }
        // let val = extrinsics(meta, &block_json).unwrap();
        // println!("{:#?}", val);
    }

    #[test]
    fn can_decode_events() {
        async_std::task::block_on(test_events());
    }

    async fn test_events() {
        let block_hash =
            hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
                .unwrap();

        let client = polkapipe::ws::Backend::new_ws2("wss://rpc.polkadot.io")
            .await
            .unwrap();
        let metadata = client.query_metadata(Some(&block_hash[..])).await.unwrap();
        let meta =
            frame_metadata::RuntimeMetadataPrefixed::decode(&mut metadata.as_slice()).unwrap();
        assert!(matches!(meta.1, RuntimeMetadata::V14(_)));

        let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
        let key = hex::decode(events_key).unwrap();

        let as_of_events = client
            .query_storage(&key[..], Some(&block_hash))
            .await
            .unwrap();
        assert!(as_of_events.len() > 0);
        println!("{:?}", as_of_events);

        let _val = decode_events(&meta, &as_of_events[..]).unwrap();
        // println!("{:#?}", val);
    }
}