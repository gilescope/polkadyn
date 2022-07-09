use frame_metadata::RuntimeMetadata;
// use scale_info::PortableRegistry;
use scale_value::Value;
mod types_that_should_be_defined_somewhere_else;
// use scale_info::PortableType;
use parity_scale_codec::Compact;
use parity_scale_codec::Decode;
use types_that_should_be_defined_somewhere_else::Phase;
pub fn events(
    metadata: frame_metadata::RuntimeMetadataPrefixed,
    scale_encoded_data: &[u8],
) -> Result<Vec<(Phase, Value<()>)>, ()> {
    if let RuntimeMetadata::V14(metadata) = metadata.1 {
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
    use crate::events;
    use frame_metadata::RuntimeMetadata;
    use parity_scale_codec::Decode;
    use polkapipe::Backend;

    #[test]
    fn it_works() {
        async_std::task::block_on(test());
    }

    async fn test() {
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

        let val = events(meta, &as_of_events[..]).unwrap();
        println!("{:#?}", val);
    }
}
