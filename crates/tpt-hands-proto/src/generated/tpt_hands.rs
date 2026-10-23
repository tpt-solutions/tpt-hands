pub mod tpt_ {
    pub mod hands_ {
        pub mod v1_ {
            /// Health of one pathway, as the edge reports it.
            #[derive(Debug, Default, PartialEq, Clone, Copy)]
            pub struct PathwayHealth {
                pub r#id: u32,
                pub r#state: PathwayState,
                /// meaningful only when state is SEVERED
                pub r#healing_progress: f32,
                pub r#resistance_ohms: f32,
                pub r#max_bandwidth_mbps: f32,
            }
            impl PathwayHealth {
                /// Return a reference to `id`
                #[inline]
                pub fn r#id(&self) -> &u32 {
                    &self.r#id
                }
                /// Return a mutable reference to `id`
                #[inline]
                pub fn mut_id(&mut self) -> &mut u32 {
                    &mut self.r#id
                }
                /// Set the value of `id`
                #[inline]
                pub fn set_id(&mut self, value: u32) -> &mut Self {
                    self.r#id = value.into();
                    self
                }
                /// Builder method that sets the value of `id`. Useful for initializing the message.
                #[inline]
                pub fn init_id(mut self, value: u32) -> Self {
                    self.r#id = value.into();
                    self
                }
                /// Return a reference to `state`
                #[inline]
                pub fn r#state(&self) -> &PathwayState {
                    &self.r#state
                }
                /// Return a mutable reference to `state`
                #[inline]
                pub fn mut_state(&mut self) -> &mut PathwayState {
                    &mut self.r#state
                }
                /// Set the value of `state`
                #[inline]
                pub fn set_state(&mut self, value: PathwayState) -> &mut Self {
                    self.r#state = value.into();
                    self
                }
                /// Builder method that sets the value of `state`. Useful for initializing the message.
                #[inline]
                pub fn init_state(mut self, value: PathwayState) -> Self {
                    self.r#state = value.into();
                    self
                }
                /// Return a reference to `healing_progress`
                #[inline]
                pub fn r#healing_progress(&self) -> &f32 {
                    &self.r#healing_progress
                }
                /// Return a mutable reference to `healing_progress`
                #[inline]
                pub fn mut_healing_progress(&mut self) -> &mut f32 {
                    &mut self.r#healing_progress
                }
                /// Set the value of `healing_progress`
                #[inline]
                pub fn set_healing_progress(&mut self, value: f32) -> &mut Self {
                    self.r#healing_progress = value.into();
                    self
                }
                /// Builder method that sets the value of `healing_progress`. Useful for initializing the message.
                #[inline]
                pub fn init_healing_progress(mut self, value: f32) -> Self {
                    self.r#healing_progress = value.into();
                    self
                }
                /// Return a reference to `resistance_ohms`
                #[inline]
                pub fn r#resistance_ohms(&self) -> &f32 {
                    &self.r#resistance_ohms
                }
                /// Return a mutable reference to `resistance_ohms`
                #[inline]
                pub fn mut_resistance_ohms(&mut self) -> &mut f32 {
                    &mut self.r#resistance_ohms
                }
                /// Set the value of `resistance_ohms`
                #[inline]
                pub fn set_resistance_ohms(&mut self, value: f32) -> &mut Self {
                    self.r#resistance_ohms = value.into();
                    self
                }
                /// Builder method that sets the value of `resistance_ohms`. Useful for initializing the message.
                #[inline]
                pub fn init_resistance_ohms(mut self, value: f32) -> Self {
                    self.r#resistance_ohms = value.into();
                    self
                }
                /// Return a reference to `max_bandwidth_mbps`
                #[inline]
                pub fn r#max_bandwidth_mbps(&self) -> &f32 {
                    &self.r#max_bandwidth_mbps
                }
                /// Return a mutable reference to `max_bandwidth_mbps`
                #[inline]
                pub fn mut_max_bandwidth_mbps(&mut self) -> &mut f32 {
                    &mut self.r#max_bandwidth_mbps
                }
                /// Set the value of `max_bandwidth_mbps`
                #[inline]
                pub fn set_max_bandwidth_mbps(&mut self, value: f32) -> &mut Self {
                    self.r#max_bandwidth_mbps = value.into();
                    self
                }
                /// Builder method that sets the value of `max_bandwidth_mbps`. Useful for initializing the message.
                #[inline]
                pub fn init_max_bandwidth_mbps(mut self, value: f32) -> Self {
                    self.r#max_bandwidth_mbps = value.into();
                    self
                }
            }
            impl ::micropb::MessageDecode for PathwayHealth {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#id;
                                {
                                    let val = decoder.decode_varint32()?;
                                    let val_ref = &val;
                                    if *val_ref != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#state;
                                {
                                    let val = decoder
                                        .decode_int32()
                                        .map(|n| PathwayState(n as _))?;
                                    let val_ref = &val;
                                    if val_ref.0 != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            3u32 => {
                                let mut_ref = &mut self.r#healing_progress;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            4u32 => {
                                let mut_ref = &mut self.r#resistance_ohms;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            5u32 => {
                                let mut_ref = &mut self.r#max_bandwidth_mbps;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for PathwayHealth {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(5usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(PathwayState::_MAX_SIZE), | size |
                        size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#id;
                        if *val_ref != 0 {
                            encoder.encode_varint32(8u32)?;
                            encoder.encode_varint32(*val_ref as _)?;
                        }
                    }
                    {
                        let val_ref = &self.r#state;
                        if val_ref.0 != 0 {
                            encoder.encode_varint32(16u32)?;
                            encoder.encode_int32(val_ref.0 as _)?;
                        }
                    }
                    {
                        let val_ref = &self.r#healing_progress;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(29u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#resistance_ohms;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(37u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#max_bandwidth_mbps;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(45u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#id;
                        if *val_ref != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_varint32(*val_ref as _);
                        }
                    }
                    {
                        let val_ref = &self.r#state;
                        if val_ref.0 != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_int32(val_ref.0 as _);
                        }
                    }
                    {
                        let val_ref = &self.r#healing_progress;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    {
                        let val_ref = &self.r#resistance_ohms;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    {
                        let val_ref = &self.r#max_bandwidth_mbps;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    size
                }
            }
            /// Physiological state, streamed to tpt-soma.
            #[derive(Debug, Default, PartialEq, Clone)]
            pub struct PhysiologicalUpdate {
                pub r#tool_id: ::alloc::string::String,
                pub r#structural_integrity: f32,
                pub r#pathways: ::alloc::vec::Vec<PathwayHealth>,
                pub r#timestamp_us: u64,
            }
            impl PhysiologicalUpdate {
                /// Return a reference to `tool_id`
                #[inline]
                pub fn r#tool_id(&self) -> &::alloc::string::String {
                    &self.r#tool_id
                }
                /// Return a mutable reference to `tool_id`
                #[inline]
                pub fn mut_tool_id(&mut self) -> &mut ::alloc::string::String {
                    &mut self.r#tool_id
                }
                /// Set the value of `tool_id`
                #[inline]
                pub fn set_tool_id(
                    &mut self,
                    value: ::alloc::string::String,
                ) -> &mut Self {
                    self.r#tool_id = value.into();
                    self
                }
                /// Builder method that sets the value of `tool_id`. Useful for initializing the message.
                #[inline]
                pub fn init_tool_id(mut self, value: ::alloc::string::String) -> Self {
                    self.r#tool_id = value.into();
                    self
                }
                /// Return a reference to `structural_integrity`
                #[inline]
                pub fn r#structural_integrity(&self) -> &f32 {
                    &self.r#structural_integrity
                }
                /// Return a mutable reference to `structural_integrity`
                #[inline]
                pub fn mut_structural_integrity(&mut self) -> &mut f32 {
                    &mut self.r#structural_integrity
                }
                /// Set the value of `structural_integrity`
                #[inline]
                pub fn set_structural_integrity(&mut self, value: f32) -> &mut Self {
                    self.r#structural_integrity = value.into();
                    self
                }
                /// Builder method that sets the value of `structural_integrity`. Useful for initializing the message.
                #[inline]
                pub fn init_structural_integrity(mut self, value: f32) -> Self {
                    self.r#structural_integrity = value.into();
                    self
                }
                /// Return a reference to `timestamp_us`
                #[inline]
                pub fn r#timestamp_us(&self) -> &u64 {
                    &self.r#timestamp_us
                }
                /// Return a mutable reference to `timestamp_us`
                #[inline]
                pub fn mut_timestamp_us(&mut self) -> &mut u64 {
                    &mut self.r#timestamp_us
                }
                /// Set the value of `timestamp_us`
                #[inline]
                pub fn set_timestamp_us(&mut self, value: u64) -> &mut Self {
                    self.r#timestamp_us = value.into();
                    self
                }
                /// Builder method that sets the value of `timestamp_us`. Useful for initializing the message.
                #[inline]
                pub fn init_timestamp_us(mut self, value: u64) -> Self {
                    self.r#timestamp_us = value.into();
                    self
                }
            }
            impl ::micropb::MessageDecode for PhysiologicalUpdate {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#tool_id;
                                {
                                    decoder
                                        .decode_string(mut_ref, ::micropb::Presence::Implicit)?;
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#structural_integrity;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            3u32 => {
                                let mut val: PathwayHealth = ::core::default::Default::default();
                                let mut_ref = &mut val;
                                {
                                    mut_ref.decode_len_delimited(decoder)?;
                                };
                                if let (Err(_), false) = (
                                    self.r#pathways.pb_push(val),
                                    decoder.ignore_repeated_cap_err,
                                ) {
                                    return Err(::micropb::DecodeError::Capacity);
                                }
                            }
                            4u32 => {
                                let mut_ref = &mut self.r#timestamp_us;
                                {
                                    let val = decoder.decode_varint64()?;
                                    let val_ref = &val;
                                    if *val_ref != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for PhysiologicalUpdate {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result:: < usize, & 'static str >
                        ::Err("(.tpt.hands.v1.PhysiologicalUpdate.tool_id) unbounded string or bytes"),
                        | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::core::result::Result::<
                        usize,
                        &'static str,
                    >::Err(
                        "(.tpt.hands.v1.PhysiologicalUpdate.pathways) unbounded vec",
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(10usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#tool_id;
                        if !val_ref.is_empty() {
                            encoder.encode_varint32(10u32)?;
                            encoder.encode_string(val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#structural_integrity;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(21u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    {
                        for val_ref in self.r#pathways.iter() {
                            encoder.encode_varint32(26u32)?;
                            val_ref.encode_len_delimited(encoder)?;
                        }
                    }
                    {
                        let val_ref = &self.r#timestamp_us;
                        if *val_ref != 0 {
                            encoder.encode_varint32(32u32)?;
                            encoder.encode_varint64(*val_ref as _)?;
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#tool_id;
                        if !val_ref.is_empty() {
                            size
                                += 1usize
                                    + ::micropb::size::sizeof_len_record(val_ref.len());
                        }
                    }
                    {
                        let val_ref = &self.r#structural_integrity;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    {
                        for val_ref in self.r#pathways.iter() {
                            size
                                += 1usize
                                    + ::micropb::size::sizeof_len_record(
                                        val_ref.compute_size(),
                                    );
                        }
                    }
                    {
                        let val_ref = &self.r#timestamp_us;
                        if *val_ref != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_varint64(*val_ref as _);
                        }
                    }
                    size
                }
            }
            #[derive(Debug, Default, PartialEq, Clone, Copy)]
            pub struct HealPathwayAction {
                pub r#pathway_id: u32,
                pub r#heat_assist: bool,
            }
            impl HealPathwayAction {
                /// Return a reference to `pathway_id`
                #[inline]
                pub fn r#pathway_id(&self) -> &u32 {
                    &self.r#pathway_id
                }
                /// Return a mutable reference to `pathway_id`
                #[inline]
                pub fn mut_pathway_id(&mut self) -> &mut u32 {
                    &mut self.r#pathway_id
                }
                /// Set the value of `pathway_id`
                #[inline]
                pub fn set_pathway_id(&mut self, value: u32) -> &mut Self {
                    self.r#pathway_id = value.into();
                    self
                }
                /// Builder method that sets the value of `pathway_id`. Useful for initializing the message.
                #[inline]
                pub fn init_pathway_id(mut self, value: u32) -> Self {
                    self.r#pathway_id = value.into();
                    self
                }
                /// Return a reference to `heat_assist`
                #[inline]
                pub fn r#heat_assist(&self) -> &bool {
                    &self.r#heat_assist
                }
                /// Return a mutable reference to `heat_assist`
                #[inline]
                pub fn mut_heat_assist(&mut self) -> &mut bool {
                    &mut self.r#heat_assist
                }
                /// Set the value of `heat_assist`
                #[inline]
                pub fn set_heat_assist(&mut self, value: bool) -> &mut Self {
                    self.r#heat_assist = value.into();
                    self
                }
                /// Builder method that sets the value of `heat_assist`. Useful for initializing the message.
                #[inline]
                pub fn init_heat_assist(mut self, value: bool) -> Self {
                    self.r#heat_assist = value.into();
                    self
                }
            }
            impl ::micropb::MessageDecode for HealPathwayAction {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#pathway_id;
                                {
                                    let val = decoder.decode_varint32()?;
                                    let val_ref = &val;
                                    if *val_ref != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#heat_assist;
                                {
                                    let val = decoder.decode_bool()?;
                                    let val_ref = &val;
                                    if *val_ref {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for HealPathwayAction {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(5usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(1usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#pathway_id;
                        if *val_ref != 0 {
                            encoder.encode_varint32(8u32)?;
                            encoder.encode_varint32(*val_ref as _)?;
                        }
                    }
                    {
                        let val_ref = &self.r#heat_assist;
                        if *val_ref {
                            encoder.encode_varint32(16u32)?;
                            encoder.encode_bool(*val_ref)?;
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#pathway_id;
                        if *val_ref != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_varint32(*val_ref as _);
                        }
                    }
                    {
                        let val_ref = &self.r#heat_assist;
                        if *val_ref {
                            size += 1usize + 1;
                        }
                    }
                    size
                }
            }
            #[derive(Debug, Default, PartialEq, Clone, Copy)]
            pub struct SproutConduitAction {
                pub r#origin: u32,
                pub r#destination: u32,
                pub r#fluid_type: FluidType,
            }
            impl SproutConduitAction {
                /// Return a reference to `origin`
                #[inline]
                pub fn r#origin(&self) -> &u32 {
                    &self.r#origin
                }
                /// Return a mutable reference to `origin`
                #[inline]
                pub fn mut_origin(&mut self) -> &mut u32 {
                    &mut self.r#origin
                }
                /// Set the value of `origin`
                #[inline]
                pub fn set_origin(&mut self, value: u32) -> &mut Self {
                    self.r#origin = value.into();
                    self
                }
                /// Builder method that sets the value of `origin`. Useful for initializing the message.
                #[inline]
                pub fn init_origin(mut self, value: u32) -> Self {
                    self.r#origin = value.into();
                    self
                }
                /// Return a reference to `destination`
                #[inline]
                pub fn r#destination(&self) -> &u32 {
                    &self.r#destination
                }
                /// Return a mutable reference to `destination`
                #[inline]
                pub fn mut_destination(&mut self) -> &mut u32 {
                    &mut self.r#destination
                }
                /// Set the value of `destination`
                #[inline]
                pub fn set_destination(&mut self, value: u32) -> &mut Self {
                    self.r#destination = value.into();
                    self
                }
                /// Builder method that sets the value of `destination`. Useful for initializing the message.
                #[inline]
                pub fn init_destination(mut self, value: u32) -> Self {
                    self.r#destination = value.into();
                    self
                }
                /// Return a reference to `fluid_type`
                #[inline]
                pub fn r#fluid_type(&self) -> &FluidType {
                    &self.r#fluid_type
                }
                /// Return a mutable reference to `fluid_type`
                #[inline]
                pub fn mut_fluid_type(&mut self) -> &mut FluidType {
                    &mut self.r#fluid_type
                }
                /// Set the value of `fluid_type`
                #[inline]
                pub fn set_fluid_type(&mut self, value: FluidType) -> &mut Self {
                    self.r#fluid_type = value.into();
                    self
                }
                /// Builder method that sets the value of `fluid_type`. Useful for initializing the message.
                #[inline]
                pub fn init_fluid_type(mut self, value: FluidType) -> Self {
                    self.r#fluid_type = value.into();
                    self
                }
            }
            impl ::micropb::MessageDecode for SproutConduitAction {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#origin;
                                {
                                    let val = decoder.decode_varint32()?;
                                    let val_ref = &val;
                                    if *val_ref != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#destination;
                                {
                                    let val = decoder.decode_varint32()?;
                                    let val_ref = &val;
                                    if *val_ref != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            3u32 => {
                                let mut_ref = &mut self.r#fluid_type;
                                {
                                    let val = decoder
                                        .decode_int32()
                                        .map(|n| FluidType(n as _))?;
                                    let val_ref = &val;
                                    if val_ref.0 != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for SproutConduitAction {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(5usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(5usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(FluidType::_MAX_SIZE), | size | size +
                        1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#origin;
                        if *val_ref != 0 {
                            encoder.encode_varint32(8u32)?;
                            encoder.encode_varint32(*val_ref as _)?;
                        }
                    }
                    {
                        let val_ref = &self.r#destination;
                        if *val_ref != 0 {
                            encoder.encode_varint32(16u32)?;
                            encoder.encode_varint32(*val_ref as _)?;
                        }
                    }
                    {
                        let val_ref = &self.r#fluid_type;
                        if val_ref.0 != 0 {
                            encoder.encode_varint32(24u32)?;
                            encoder.encode_int32(val_ref.0 as _)?;
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#origin;
                        if *val_ref != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_varint32(*val_ref as _);
                        }
                    }
                    {
                        let val_ref = &self.r#destination;
                        if *val_ref != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_varint32(*val_ref as _);
                        }
                    }
                    {
                        let val_ref = &self.r#fluid_type;
                        if val_ref.0 != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_int32(val_ref.0 as _);
                        }
                    }
                    size
                }
            }
            #[derive(Debug, Default, PartialEq, Clone, Copy)]
            pub struct ReinforceTendonAction {
                pub r#tendon_id: u32,
                pub r#volume_ml: f32,
            }
            impl ReinforceTendonAction {
                /// Return a reference to `tendon_id`
                #[inline]
                pub fn r#tendon_id(&self) -> &u32 {
                    &self.r#tendon_id
                }
                /// Return a mutable reference to `tendon_id`
                #[inline]
                pub fn mut_tendon_id(&mut self) -> &mut u32 {
                    &mut self.r#tendon_id
                }
                /// Set the value of `tendon_id`
                #[inline]
                pub fn set_tendon_id(&mut self, value: u32) -> &mut Self {
                    self.r#tendon_id = value.into();
                    self
                }
                /// Builder method that sets the value of `tendon_id`. Useful for initializing the message.
                #[inline]
                pub fn init_tendon_id(mut self, value: u32) -> Self {
                    self.r#tendon_id = value.into();
                    self
                }
                /// Return a reference to `volume_ml`
                #[inline]
                pub fn r#volume_ml(&self) -> &f32 {
                    &self.r#volume_ml
                }
                /// Return a mutable reference to `volume_ml`
                #[inline]
                pub fn mut_volume_ml(&mut self) -> &mut f32 {
                    &mut self.r#volume_ml
                }
                /// Set the value of `volume_ml`
                #[inline]
                pub fn set_volume_ml(&mut self, value: f32) -> &mut Self {
                    self.r#volume_ml = value.into();
                    self
                }
                /// Builder method that sets the value of `volume_ml`. Useful for initializing the message.
                #[inline]
                pub fn init_volume_ml(mut self, value: f32) -> Self {
                    self.r#volume_ml = value.into();
                    self
                }
            }
            impl ::micropb::MessageDecode for ReinforceTendonAction {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#tendon_id;
                                {
                                    let val = decoder.decode_varint32()?;
                                    let val_ref = &val;
                                    if *val_ref != 0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#volume_ml;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for ReinforceTendonAction {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(5usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#tendon_id;
                        if *val_ref != 0 {
                            encoder.encode_varint32(8u32)?;
                            encoder.encode_varint32(*val_ref as _)?;
                        }
                    }
                    {
                        let val_ref = &self.r#volume_ml;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(21u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#tendon_id;
                        if *val_ref != 0 {
                            size
                                += 1usize + ::micropb::size::sizeof_varint32(*val_ref as _);
                        }
                    }
                    {
                        let val_ref = &self.r#volume_ml;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    size
                }
            }
            #[derive(Debug, Default, PartialEq, Clone, Copy)]
            pub struct SafetyEnvelope {
                pub r#max_pressure_kpa: f32,
                pub r#max_temperature_c: f32,
                pub r#max_volume_ml: f32,
            }
            impl SafetyEnvelope {
                /// Return a reference to `max_pressure_kpa`
                #[inline]
                pub fn r#max_pressure_kpa(&self) -> &f32 {
                    &self.r#max_pressure_kpa
                }
                /// Return a mutable reference to `max_pressure_kpa`
                #[inline]
                pub fn mut_max_pressure_kpa(&mut self) -> &mut f32 {
                    &mut self.r#max_pressure_kpa
                }
                /// Set the value of `max_pressure_kpa`
                #[inline]
                pub fn set_max_pressure_kpa(&mut self, value: f32) -> &mut Self {
                    self.r#max_pressure_kpa = value.into();
                    self
                }
                /// Builder method that sets the value of `max_pressure_kpa`. Useful for initializing the message.
                #[inline]
                pub fn init_max_pressure_kpa(mut self, value: f32) -> Self {
                    self.r#max_pressure_kpa = value.into();
                    self
                }
                /// Return a reference to `max_temperature_c`
                #[inline]
                pub fn r#max_temperature_c(&self) -> &f32 {
                    &self.r#max_temperature_c
                }
                /// Return a mutable reference to `max_temperature_c`
                #[inline]
                pub fn mut_max_temperature_c(&mut self) -> &mut f32 {
                    &mut self.r#max_temperature_c
                }
                /// Set the value of `max_temperature_c`
                #[inline]
                pub fn set_max_temperature_c(&mut self, value: f32) -> &mut Self {
                    self.r#max_temperature_c = value.into();
                    self
                }
                /// Builder method that sets the value of `max_temperature_c`. Useful for initializing the message.
                #[inline]
                pub fn init_max_temperature_c(mut self, value: f32) -> Self {
                    self.r#max_temperature_c = value.into();
                    self
                }
                /// Return a reference to `max_volume_ml`
                #[inline]
                pub fn r#max_volume_ml(&self) -> &f32 {
                    &self.r#max_volume_ml
                }
                /// Return a mutable reference to `max_volume_ml`
                #[inline]
                pub fn mut_max_volume_ml(&mut self) -> &mut f32 {
                    &mut self.r#max_volume_ml
                }
                /// Set the value of `max_volume_ml`
                #[inline]
                pub fn set_max_volume_ml(&mut self, value: f32) -> &mut Self {
                    self.r#max_volume_ml = value.into();
                    self
                }
                /// Builder method that sets the value of `max_volume_ml`. Useful for initializing the message.
                #[inline]
                pub fn init_max_volume_ml(mut self, value: f32) -> Self {
                    self.r#max_volume_ml = value.into();
                    self
                }
            }
            impl ::micropb::MessageDecode for SafetyEnvelope {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#max_pressure_kpa;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#max_temperature_c;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            3u32 => {
                                let mut_ref = &mut self.r#max_volume_ml;
                                {
                                    let val = decoder.decode_float()?;
                                    let val_ref = &val;
                                    if *val_ref != 0.0 {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for SafetyEnvelope {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(4usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#max_pressure_kpa;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(13u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#max_temperature_c;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(21u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#max_volume_ml;
                        if *val_ref != 0.0 {
                            encoder.encode_varint32(29u32)?;
                            encoder.encode_float(*val_ref)?;
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#max_pressure_kpa;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    {
                        let val_ref = &self.r#max_temperature_c;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    {
                        let val_ref = &self.r#max_volume_ml;
                        if *val_ref != 0.0 {
                            size += 1usize + 4;
                        }
                    }
                    size
                }
            }
            /// Request from tpt-hands to tpt-chrysalis. It never actuates pumps directly.
            #[derive(Debug, Default, Clone)]
            pub struct MorphogenesisRequest {
                pub r#tool_id: ::alloc::string::String,
                /// GAP: DESIGN.md has no request id, but MorphogenesisAck refers to one.
                /// Added here so the ack can be matched to its request.
                pub r#request_id: ::alloc::string::String,
                /// *Note:* The presence of this field is tracked separately in the `_has` field. It's recommended to access this field via the accessor rather than directly.
                pub r#safety_limits: SafetyEnvelope,
                pub r#action: ::core::option::Option<MorphogenesisRequest_::Action>,
                /// Tracks presence of optional and message fields
                pub _has: MorphogenesisRequest_::_Hazzer,
            }
            impl ::core::cmp::PartialEq for MorphogenesisRequest {
                fn eq(&self, other: &Self) -> bool {
                    let mut ret = true;
                    ret &= (self.r#tool_id == other.r#tool_id);
                    ret &= (self.r#request_id == other.r#request_id);
                    ret &= (self.r#safety_limits() == other.r#safety_limits());
                    ret &= (self.r#action == other.r#action);
                    ret
                }
            }
            impl MorphogenesisRequest {
                /// Return a reference to `tool_id`
                #[inline]
                pub fn r#tool_id(&self) -> &::alloc::string::String {
                    &self.r#tool_id
                }
                /// Return a mutable reference to `tool_id`
                #[inline]
                pub fn mut_tool_id(&mut self) -> &mut ::alloc::string::String {
                    &mut self.r#tool_id
                }
                /// Set the value of `tool_id`
                #[inline]
                pub fn set_tool_id(
                    &mut self,
                    value: ::alloc::string::String,
                ) -> &mut Self {
                    self.r#tool_id = value.into();
                    self
                }
                /// Builder method that sets the value of `tool_id`. Useful for initializing the message.
                #[inline]
                pub fn init_tool_id(mut self, value: ::alloc::string::String) -> Self {
                    self.r#tool_id = value.into();
                    self
                }
                /// Return a reference to `request_id`
                #[inline]
                pub fn r#request_id(&self) -> &::alloc::string::String {
                    &self.r#request_id
                }
                /// Return a mutable reference to `request_id`
                #[inline]
                pub fn mut_request_id(&mut self) -> &mut ::alloc::string::String {
                    &mut self.r#request_id
                }
                /// Set the value of `request_id`
                #[inline]
                pub fn set_request_id(
                    &mut self,
                    value: ::alloc::string::String,
                ) -> &mut Self {
                    self.r#request_id = value.into();
                    self
                }
                /// Builder method that sets the value of `request_id`. Useful for initializing the message.
                #[inline]
                pub fn init_request_id(
                    mut self,
                    value: ::alloc::string::String,
                ) -> Self {
                    self.r#request_id = value.into();
                    self
                }
                /// Return a reference to `safety_limits` as an `Option`
                #[inline]
                pub fn r#safety_limits(
                    &self,
                ) -> ::core::option::Option<&SafetyEnvelope> {
                    self._has.r#safety_limits().then_some(&self.r#safety_limits)
                }
                /// Set the value and presence of `safety_limits`
                #[inline]
                pub fn set_safety_limits(&mut self, value: SafetyEnvelope) -> &mut Self {
                    self._has.set_safety_limits();
                    self.r#safety_limits = value.into();
                    self
                }
                /// Return a mutable reference to `safety_limits` as an `Option`
                #[inline]
                pub fn mut_safety_limits(
                    &mut self,
                ) -> ::core::option::Option<&mut SafetyEnvelope> {
                    self._has.r#safety_limits().then_some(&mut self.r#safety_limits)
                }
                /// Clear the presence of `safety_limits`
                #[inline]
                pub fn clear_safety_limits(&mut self) -> &mut Self {
                    self._has.clear_safety_limits();
                    self
                }
                /// Take the value of `safety_limits` and clear its presence
                #[inline]
                pub fn take_safety_limits(
                    &mut self,
                ) -> ::core::option::Option<SafetyEnvelope> {
                    let val = self
                        ._has
                        .r#safety_limits()
                        .then(|| ::core::mem::take(&mut self.r#safety_limits));
                    self._has.clear_safety_limits();
                    val
                }
                /// Builder method that sets the value of `safety_limits`. Useful for initializing the message.
                #[inline]
                pub fn init_safety_limits(mut self, value: SafetyEnvelope) -> Self {
                    self.set_safety_limits(value);
                    self
                }
            }
            impl ::micropb::MessageDecode for MorphogenesisRequest {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#tool_id;
                                {
                                    decoder
                                        .decode_string(mut_ref, ::micropb::Presence::Implicit)?;
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#request_id;
                                {
                                    decoder
                                        .decode_string(mut_ref, ::micropb::Presence::Implicit)?;
                                };
                            }
                            99u32 => {
                                let mut_ref = &mut self.r#safety_limits;
                                {
                                    mut_ref.decode_len_delimited(decoder)?;
                                };
                                self._has.set_safety_limits();
                            }
                            10u32 => {
                                let mut_ref = loop {
                                    if let ::core::option::Option::Some(variant) = &mut self
                                        .r#action
                                    {
                                        if let MorphogenesisRequest_::Action::Heal(variant) = &mut *variant {
                                            break &mut *variant;
                                        }
                                    }
                                    self.r#action = ::core::option::Option::Some(
                                        MorphogenesisRequest_::Action::Heal(
                                            ::core::default::Default::default(),
                                        ),
                                    );
                                };
                                mut_ref.decode_len_delimited(decoder)?;
                            }
                            11u32 => {
                                let mut_ref = loop {
                                    if let ::core::option::Option::Some(variant) = &mut self
                                        .r#action
                                    {
                                        if let MorphogenesisRequest_::Action::Sprout(variant) = &mut *variant {
                                            break &mut *variant;
                                        }
                                    }
                                    self.r#action = ::core::option::Option::Some(
                                        MorphogenesisRequest_::Action::Sprout(
                                            ::core::default::Default::default(),
                                        ),
                                    );
                                };
                                mut_ref.decode_len_delimited(decoder)?;
                            }
                            12u32 => {
                                let mut_ref = loop {
                                    if let ::core::option::Option::Some(variant) = &mut self
                                        .r#action
                                    {
                                        if let MorphogenesisRequest_::Action::Reinforce(variant) = &mut *variant {
                                            break &mut *variant;
                                        }
                                    }
                                    self.r#action = ::core::option::Option::Some(
                                        MorphogenesisRequest_::Action::Reinforce(
                                            ::core::default::Default::default(),
                                        ),
                                    );
                                };
                                mut_ref.decode_len_delimited(decoder)?;
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for MorphogenesisRequest {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result:: < usize, & 'static str >
                        ::Err("(.tpt.hands.v1.MorphogenesisRequest.tool_id) unbounded string or bytes"),
                        | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result:: < usize, & 'static str >
                        ::Err("(.tpt.hands.v1.MorphogenesisRequest.request_id) unbounded string or bytes"),
                        | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::micropb::const_map!(< SafetyEnvelope as
                        ::micropb::MessageEncode > ::MAX_SIZE, | size |
                        ::micropb::size::sizeof_len_record(size)), | size | size + 2usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match 'oneof: {
                        let mut max_size = 0;
                        match ::micropb::const_map!(
                            ::micropb::const_map!(< HealPathwayAction as
                            ::micropb::MessageEncode > ::MAX_SIZE, | size |
                            ::micropb::size::sizeof_len_record(size)), | size | size +
                            1usize
                        ) {
                            ::core::result::Result::Ok(size) => {
                                if size > max_size {
                                    max_size = size;
                                }
                            }
                            ::core::result::Result::Err(err) => {
                                break 'oneof (::core::result::Result::<usize, _>::Err(err));
                            }
                        }
                        match ::micropb::const_map!(
                            ::micropb::const_map!(< SproutConduitAction as
                            ::micropb::MessageEncode > ::MAX_SIZE, | size |
                            ::micropb::size::sizeof_len_record(size)), | size | size +
                            1usize
                        ) {
                            ::core::result::Result::Ok(size) => {
                                if size > max_size {
                                    max_size = size;
                                }
                            }
                            ::core::result::Result::Err(err) => {
                                break 'oneof (::core::result::Result::<usize, _>::Err(err));
                            }
                        }
                        match ::micropb::const_map!(
                            ::micropb::const_map!(< ReinforceTendonAction as
                            ::micropb::MessageEncode > ::MAX_SIZE, | size |
                            ::micropb::size::sizeof_len_record(size)), | size | size +
                            1usize
                        ) {
                            ::core::result::Result::Ok(size) => {
                                if size > max_size {
                                    max_size = size;
                                }
                            }
                            ::core::result::Result::Err(err) => {
                                break 'oneof (::core::result::Result::<usize, _>::Err(err));
                            }
                        }
                        ::core::result::Result::Ok(max_size)
                    } {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#tool_id;
                        if !val_ref.is_empty() {
                            encoder.encode_varint32(10u32)?;
                            encoder.encode_string(val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#request_id;
                        if !val_ref.is_empty() {
                            encoder.encode_varint32(18u32)?;
                            encoder.encode_string(val_ref)?;
                        }
                    }
                    {
                        if let ::core::option::Option::Some(val_ref) = self
                            .r#safety_limits()
                        {
                            encoder.encode_varint32(794u32)?;
                            val_ref.encode_len_delimited(encoder)?;
                        }
                    }
                    if let Some(oneof) = &self.r#action {
                        match &*oneof {
                            MorphogenesisRequest_::Action::Heal(val_ref) => {
                                let val_ref = &*val_ref;
                                encoder.encode_varint32(82u32)?;
                                val_ref.encode_len_delimited(encoder)?;
                            }
                            MorphogenesisRequest_::Action::Sprout(val_ref) => {
                                let val_ref = &*val_ref;
                                encoder.encode_varint32(90u32)?;
                                val_ref.encode_len_delimited(encoder)?;
                            }
                            MorphogenesisRequest_::Action::Reinforce(val_ref) => {
                                let val_ref = &*val_ref;
                                encoder.encode_varint32(98u32)?;
                                val_ref.encode_len_delimited(encoder)?;
                            }
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#tool_id;
                        if !val_ref.is_empty() {
                            size
                                += 1usize
                                    + ::micropb::size::sizeof_len_record(val_ref.len());
                        }
                    }
                    {
                        let val_ref = &self.r#request_id;
                        if !val_ref.is_empty() {
                            size
                                += 1usize
                                    + ::micropb::size::sizeof_len_record(val_ref.len());
                        }
                    }
                    {
                        if let ::core::option::Option::Some(val_ref) = self
                            .r#safety_limits()
                        {
                            size
                                += 2usize
                                    + ::micropb::size::sizeof_len_record(
                                        val_ref.compute_size(),
                                    );
                        }
                    }
                    if let Some(oneof) = &self.r#action {
                        match &*oneof {
                            MorphogenesisRequest_::Action::Heal(val_ref) => {
                                let val_ref = &*val_ref;
                                size
                                    += 1usize
                                        + ::micropb::size::sizeof_len_record(
                                            val_ref.compute_size(),
                                        );
                            }
                            MorphogenesisRequest_::Action::Sprout(val_ref) => {
                                let val_ref = &*val_ref;
                                size
                                    += 1usize
                                        + ::micropb::size::sizeof_len_record(
                                            val_ref.compute_size(),
                                        );
                            }
                            MorphogenesisRequest_::Action::Reinforce(val_ref) => {
                                let val_ref = &*val_ref;
                                size
                                    += 1usize
                                        + ::micropb::size::sizeof_len_record(
                                            val_ref.compute_size(),
                                        );
                            }
                        }
                    }
                    size
                }
            }
            /// Inner types for `MorphogenesisRequest`
            pub mod MorphogenesisRequest_ {
                #[derive(Debug, PartialEq, Clone)]
                pub enum Action {
                    Heal(super::HealPathwayAction),
                    Sprout(super::SproutConduitAction),
                    Reinforce(super::ReinforceTendonAction),
                }
                /// Compact bitfield for tracking presence of optional and message fields
                #[derive(Debug, Default, PartialEq, Clone, Copy)]
                pub struct _Hazzer([u8; 1]);
                impl _Hazzer {
                    /// New hazzer with all fields set to off
                    #[inline]
                    pub const fn _new() -> Self {
                        Self([0; 1])
                    }
                    /// Query presence of `safety_limits`
                    #[inline]
                    pub const fn r#safety_limits(&self) -> bool {
                        (self.0[0] & 1) != 0
                    }
                    /// Set presence of `safety_limits`
                    #[inline]
                    pub const fn set_safety_limits(&mut self) -> &mut Self {
                        let elem = &mut self.0[0];
                        *elem |= 1;
                        self
                    }
                    /// Clear presence of `safety_limits`
                    #[inline]
                    pub const fn clear_safety_limits(&mut self) -> &mut Self {
                        let elem = &mut self.0[0];
                        *elem &= !1;
                        self
                    }
                    /// Builder method that sets the presence of `safety_limits`. Useful for initializing the Hazzer.
                    #[inline]
                    pub const fn init_safety_limits(mut self) -> Self {
                        self.set_safety_limits();
                        self
                    }
                }
            }
            /// Reply from tpt-chrysalis.
            #[derive(Debug, Default, PartialEq, Clone)]
            pub struct MorphogenesisAck {
                pub r#request_id: ::alloc::string::String,
                pub r#approved: bool,
                /// tracked by tpt-chrysalis
                pub r#chrysalis_job_id: ::alloc::string::String,
            }
            impl MorphogenesisAck {
                /// Return a reference to `request_id`
                #[inline]
                pub fn r#request_id(&self) -> &::alloc::string::String {
                    &self.r#request_id
                }
                /// Return a mutable reference to `request_id`
                #[inline]
                pub fn mut_request_id(&mut self) -> &mut ::alloc::string::String {
                    &mut self.r#request_id
                }
                /// Set the value of `request_id`
                #[inline]
                pub fn set_request_id(
                    &mut self,
                    value: ::alloc::string::String,
                ) -> &mut Self {
                    self.r#request_id = value.into();
                    self
                }
                /// Builder method that sets the value of `request_id`. Useful for initializing the message.
                #[inline]
                pub fn init_request_id(
                    mut self,
                    value: ::alloc::string::String,
                ) -> Self {
                    self.r#request_id = value.into();
                    self
                }
                /// Return a reference to `approved`
                #[inline]
                pub fn r#approved(&self) -> &bool {
                    &self.r#approved
                }
                /// Return a mutable reference to `approved`
                #[inline]
                pub fn mut_approved(&mut self) -> &mut bool {
                    &mut self.r#approved
                }
                /// Set the value of `approved`
                #[inline]
                pub fn set_approved(&mut self, value: bool) -> &mut Self {
                    self.r#approved = value.into();
                    self
                }
                /// Builder method that sets the value of `approved`. Useful for initializing the message.
                #[inline]
                pub fn init_approved(mut self, value: bool) -> Self {
                    self.r#approved = value.into();
                    self
                }
                /// Return a reference to `chrysalis_job_id`
                #[inline]
                pub fn r#chrysalis_job_id(&self) -> &::alloc::string::String {
                    &self.r#chrysalis_job_id
                }
                /// Return a mutable reference to `chrysalis_job_id`
                #[inline]
                pub fn mut_chrysalis_job_id(&mut self) -> &mut ::alloc::string::String {
                    &mut self.r#chrysalis_job_id
                }
                /// Set the value of `chrysalis_job_id`
                #[inline]
                pub fn set_chrysalis_job_id(
                    &mut self,
                    value: ::alloc::string::String,
                ) -> &mut Self {
                    self.r#chrysalis_job_id = value.into();
                    self
                }
                /// Builder method that sets the value of `chrysalis_job_id`. Useful for initializing the message.
                #[inline]
                pub fn init_chrysalis_job_id(
                    mut self,
                    value: ::alloc::string::String,
                ) -> Self {
                    self.r#chrysalis_job_id = value.into();
                    self
                }
            }
            impl ::micropb::MessageDecode for MorphogenesisAck {
                fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                    &mut self,
                    decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                    len: usize,
                ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                    use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                    let before = decoder.bytes_read();
                    while decoder.bytes_read() - before < len {
                        let tag = decoder.decode_tag()?;
                        match tag.field_num() {
                            0 => return Err(::micropb::DecodeError::ZeroField),
                            1u32 => {
                                let mut_ref = &mut self.r#request_id;
                                {
                                    decoder
                                        .decode_string(mut_ref, ::micropb::Presence::Implicit)?;
                                };
                            }
                            2u32 => {
                                let mut_ref = &mut self.r#approved;
                                {
                                    let val = decoder.decode_bool()?;
                                    let val_ref = &val;
                                    if *val_ref {
                                        *mut_ref = val as _;
                                    }
                                };
                            }
                            3u32 => {
                                let mut_ref = &mut self.r#chrysalis_job_id;
                                {
                                    decoder
                                        .decode_string(mut_ref, ::micropb::Presence::Implicit)?;
                                };
                            }
                            _ => {
                                decoder.skip_wire_value(tag.wire_type())?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            impl ::micropb::MessageEncode for MorphogenesisAck {
                const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                    let mut max_size = 0;
                    match ::micropb::const_map!(
                        ::core::result::Result:: < usize, & 'static str >
                        ::Err("(.tpt.hands.v1.MorphogenesisAck.request_id) unbounded string or bytes"),
                        | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result::Ok(1usize), | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    match ::micropb::const_map!(
                        ::core::result::Result:: < usize, & 'static str >
                        ::Err("(.tpt.hands.v1.MorphogenesisAck.chrysalis_job_id) unbounded string or bytes"),
                        | size | size + 1usize
                    ) {
                        ::core::result::Result::Ok(size) => {
                            max_size += size;
                        }
                        ::core::result::Result::Err(err) => {
                            break 'msg (::core::result::Result::<usize, _>::Err(err));
                        }
                    }
                    ::core::result::Result::Ok(max_size)
                };
                fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                    &self,
                    encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
                ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                    use ::micropb::{PbMap, FieldEncode};
                    {
                        let val_ref = &self.r#request_id;
                        if !val_ref.is_empty() {
                            encoder.encode_varint32(10u32)?;
                            encoder.encode_string(val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#approved;
                        if *val_ref {
                            encoder.encode_varint32(16u32)?;
                            encoder.encode_bool(*val_ref)?;
                        }
                    }
                    {
                        let val_ref = &self.r#chrysalis_job_id;
                        if !val_ref.is_empty() {
                            encoder.encode_varint32(26u32)?;
                            encoder.encode_string(val_ref)?;
                        }
                    }
                    Ok(())
                }
                fn compute_size(&self) -> usize {
                    use ::micropb::{PbMap, FieldEncode};
                    let mut size = 0;
                    {
                        let val_ref = &self.r#request_id;
                        if !val_ref.is_empty() {
                            size
                                += 1usize
                                    + ::micropb::size::sizeof_len_record(val_ref.len());
                        }
                    }
                    {
                        let val_ref = &self.r#approved;
                        if *val_ref {
                            size += 1usize + 1;
                        }
                    }
                    {
                        let val_ref = &self.r#chrysalis_job_id;
                        if !val_ref.is_empty() {
                            size
                                += 1usize
                                    + ::micropb::size::sizeof_len_record(val_ref.len());
                        }
                    }
                    size
                }
            }
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            #[repr(transparent)]
            pub struct PathwayState(pub i32);
            impl PathwayState {
                /// Maximum encoded size of the enum
                pub const _MAX_SIZE: usize = 10usize;
                pub const Intact: Self = Self(0);
                pub const Severed: Self = Self(1);
                pub const Hypertrophied: Self = Self(2);
            }
            impl core::default::Default for PathwayState {
                fn default() -> Self {
                    Self(0)
                }
            }
            impl core::convert::From<i32> for PathwayState {
                fn from(val: i32) -> Self {
                    Self(val)
                }
            }
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            #[repr(transparent)]
            pub struct FluidType(pub i32);
            impl FluidType {
                /// Maximum encoded size of the enum
                pub const _MAX_SIZE: usize = 10usize;
                /// proto3 needs a zero value; not a valid fluid
                pub const Unspecified: Self = Self(0);
                pub const SelfHealingElastomer: Self = Self(1);
                pub const LiquidMetal: Self = Self(2);
                pub const Hydrogel: Self = Self(3);
            }
            impl core::default::Default for FluidType {
                fn default() -> Self {
                    Self(0)
                }
            }
            impl core::convert::From<i32> for FluidType {
                fn from(val: i32) -> Self {
                    Self(val)
                }
            }
        }
    }
}
