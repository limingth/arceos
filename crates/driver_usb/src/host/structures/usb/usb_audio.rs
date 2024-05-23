// // USB Audio class interface protocol numbers
// const USB_PROTO_AUDIO_VER_100: u8 = 0x00;
// const USB_PROTO_AUDIO_VER_200: u8 = 0x20;

// // USB Audio class unit IDs
// const USB_AUDIO_UNDEFINED_UNIT_ID: u8 = 0;
// const USB_AUDIO_MAXIMUM_UNIT_ID: u8 = 255;

// // Audio class endpoint descriptor (v1.00 only)
// #[repr(C, packed)]
// pub(crate) struct TUSBAudioEndpointDescriptor {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_endpoint_address: u8,
//     bm_attributes: u8,
//     w_max_packet_size: u16,
//     b_interval: u8,
//     b_refresh: u8,
//     b_synch_address: u8,
// }

// // MIDI-streaming class-specific endpoint descriptor (v1.00 only)
// #[repr(C, packed)]
// pub(crate) struct TUSBMIDIStreamingEndpointDescriptor {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_sub_type: u8,
//     b_num_emb_midijack: u8,
//     b_assoc_jack_ids: [u8; 1],
// }

// impl TUSBMIDIStreamingEndpointDescriptor {
//     pub(crate) const USB_MIDI_STREAMING_SUBTYPE_GENERAL: u8 = 0x01;
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBMIDIStreamingInterfaceDescriptorHeader {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_subtype: u8,
//     bcd_adc: u16,
//     w_total_length: u16,
// }

// impl TUSBMIDIStreamingInterfaceDescriptorHeader {
//     pub(crate) const USB_MIDI_STREAMING_IFACE_SUBTYPE_HEADER: u8 = 0x01;
//     pub(crate) const USB_MIDI_STREAMING_IFACE_BCDADC_100: u16 = 0x100;
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBMIDIStreamingInterfaceDescriptorInJack {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_subtype: u8,
//     b_jack_type: u8,
//     b_jack_id: u8,
//     i_jack: u8,
// }

// impl TUSBMIDIStreamingInterfaceDescriptorInJack {
//     pub(crate) const USB_MIDI_STREAMING_IFACE_SUBTYPE_MIDI_IN_JACK: u8 = 0x02;
//     pub(crate) const USB_MIDI_STREAMING_IFACE_JACKTYPE_EMBEDDED: u8 = 0x01;
//     pub(crate) const USB_MIDI_STREAMING_IFACE_JACKTYPE_EXTERNAL: u8 = 0x02;
// }

// #[repr(C, packed)]
// struct TUSBMIDIStreamingInterfaceDescriptorOutJack {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_subtype: u8,
//     b_jack_type: u8,
//     b_jack_id: u8,
//     b_nr_input_pins: u8,
//     ba_source_id: [u8; 1],
//     ba_source_pin: [u8; 1],
//     i_jack: u8,
// }

// impl TUSBMIDIStreamingInterfaceDescriptorOutJack {
//     pub(crate) const USB_MIDI_STREAMING_IFACE_SUBTYPE_MIDI_OUT_JACK: u8 = 0x03;
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBAudioControlInterfaceDescriptorHeader {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_subtype: u8,
//     bcd_adc: u16,
//     w_total_length: u16,
//     b_in_collection: u8,
//     ba_interface_nr: [u8; 1],
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBAudioControlInterfaceDescriptor {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_subtype: u8,
//     ver100: Ver100,
//     ver200: Ver200,
// }

// impl TUSBAudioControlInterfaceDescriptor {
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_HEADER: u8 = 0x01;
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_INPUT_TERMINAL: u8 = 0x02;
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_OUTPUT_TERMINAL: u8 = 0x03;
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_MIXER_UNIT: u8 = 0x04;
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_SELECTOR_UNIT: u8 = 0x05;
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_FEATURE_UNIT: u8 = 0x06;
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_CLOCK_SOURCE: u8 = 0x0A;
//     pub(crate) const USB_AUDIO_CTL_IFACE_SUBTYPE_CLOCK_SELECTOR: u8 = 0x0B;
// }

// #[repr(C, packed)]
// pub(crate) struct SubHeaderV100 {
//     bcd_adc: u16,
//     w_total_length: u16,
//     b_in_collection: u8,
//     ba_interface_nr: [u8],
// }

// impl SubHeaderV100 {
//     pub(crate) const USB_AUDIO_CTL_IFACE_BCDADC_100: u16 = 0x100;
// }

// #[repr(C, packed)]
// pub(crate) struct SubInputTerminalV100 {
//     b_terminal_id: u8,
//     w_terminal_type: u16,
//     b_assoc_terminal: u8,
//     b_nr_channels: u8,
//     w_channel_config: u16,
//     i_channel_names: u8,
//     i_terminal: u8,
// }

// impl SubInputTerminalV100 {
//     pub(crate) const USB_AUDIO_TERMINAL_TYPE_USB_UNDEFINED: u16 = 0x100;
//     pub(crate) const USB_AUDIO_TERMINAL_TYPE_USB_STREAMING: u16 = 0x101;
//     pub(crate) const USB_AUDIO_TERMINAL_TYPE_SPEAKER: u16 = 0x301;
//     pub(crate) const USB_AUDIO_TERMINAL_TYPE_SPDIF: u16 = 0x605;
// }

// #[repr(C, packed)]
// pub(crate) struct SubOutputTerminalV100 {
//     b_terminal_id: u8,
//     w_terminal_type: u16,
//     b_assoc_terminal: u8,
//     b_source_id: u8,
//     i_terminal: u8,
// }

// #[repr(C, packed)]
// pub(crate) struct SubMixerUnitV100 {
//     b_unit_id: u8,
//     b_nr_in_pins: u8,
//     ba_source_id: [u8],
// }

// #[repr(C, packed)]
// pub(crate) struct SubSelectorUnitV100 {
//     b_unit_id: u8,
//     b_nr_in_pins: u8,
//     ba_source_id: [u8],
// }

// #[repr(C, packed)]
// pub(crate) struct SubFeatureUnitV100 {
//     b_unit_id: u8,
//     b_source_id: u8,
//     b_control_size: u8,
//     bma_controls: [u8],
// }

// pub(crate) union Ver100 {
//     head: SubHeaderV100,
//     input_terminal: SubInputTerminalV100,
//     output_terminal: SubOutputTerminalV100,
//     mixer_unit: SubMixerUnitV100,
//     selector_unit: SubSelectorUnitV100,
//     feature_unit: SubFeatureUnitV100,
// }

// #[repr(C, packed)]
// pub(crate) struct SubHeaderV200 {
//     bcd_adc: u16,
//     b_category: u8,
//     w_total_length: u16,
//     bm_controls: u8,
// }

// impl SubHeaderV200 {
//     const USB_AUDIO_CTL_IFACE_BCDADC_200: u16 = 0x200;
// }

// #[repr(C, packed)]
// pub(crate) struct SubInputTerminalV200 {
//     b_terminal_id: u8,
//     w_terminal_type: u16,
//     b_assoc_terminal: u8,
//     b_c_source_id: u8,
//     b_nr_channels: u8,
//     bm_channel_config: u32,
//     i_channel_names: u8,
//     bm_controls: u16,
//     i_terminal: u8,
// }

// #[repr(C, packed)]
// pub(crate) struct SubOutputTerminalV200 {
//     b_terminal_id: u8,
//     w_terminal_type: u16,
//     b_assoc_terminal: u8,
//     b_source_id: u8,
//     b_c_source_id: u8,
//     bm_controls: u16,
//     i_terminal: u8,
// }

// #[repr(C, packed)]
// pub(crate) struct SubMixerUnitV200 {
//     b_unit_id: u8,
//     b_nr_in_pins: u8,
//     ba_source_id: [u8],
// }

// #[repr(C, packed)]
// pub(crate) struct SelectorUnit200 {
//     b_unit_id: u8,
//     b_nr_in_pins: u8,
//     ba_source_id: [u8],
// }

// #[repr(C, packed)]
// pub(crate) struct SubFeatureUnitV200 {
//     b_unit_id: u8,
//     b_source_id: u8,
//     bma_controls: [u32],
// }

// #[repr(C, packed)]
// pub(crate) struct SubClockSourceV200 {
//     b_clock_id: u8,
//     bm_attributes: u8,
//     bm_controls: u8,
//     b_assoc_terminal: u8,
//     i_clock_source: u8,
// }

// #[repr(C, packed)]
// pub(crate) struct SubClockSelectorV200 {
//     b_clock_id: u8,
//     b_nr_in_pins: u8,
//     ba_c_source_id: [u8],
// }

// pub(crate) union Ver200 {
//     head: SubHeaderV200,
//     input_terminal: SubInputTerminalV200,
//     output_terminal: SubOutputTerminalV200,
//     mixer_unit: SubMixerUnitV200,
//     selector_unit: SelectorUnit200,
//     feature_unit: SubFeatureUnitV200,
//     clock_source: SubClockSourceV200,
//     clock_selector: SubClockSelectorV200,
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBAudioControlMixerUnitTrailerVer100 {
//     b_nr_channels: u8,
//     w_channel_config: u16,
//     i_channel_names: u8,
//     bm_controls: [u8],
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBAudioControlMixerUnitTrailerVer200 {
//     b_nr_channels: u8,
//     bm_channel_config: u32,
//     i_channel_names: u8,
//     bm_mixer_controls: [u8],
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBAudioStreamingInterfaceDescriptor {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_subtype: u8,
// }

// impl TUSBAudioStreamingInterfaceDescriptor {
//     const USB_AUDIO_STREAMING_GENERAL: u16 = 0x01;
// }

// #[repr(C, packed)]
// #[derive(Clone, Copy)]
// pub(crate) struct SubTUSBAudioStreamingInterfaceDescriptorVer100 {
//     b_terminal_link: u8,
//     b_delay: u8,
//     w_format_tag: u16,
// }

// #[repr(C, packed)]
// #[derive(Clone, Copy)]
// pub(crate) struct SubTUSBAudioStreamingInterfaceDescriptorVer200 {
//     b_terminal_link: u8,
//     b_delay: u8,
//     b_format_type: u8,
//     bm_formats: u32,
//     b_nr_channels: u8,
//     bm_channel_config: u32,
//     i_channel_names: u8,
// }

// pub(crate) union SubInterfaceDescriptorVerUnion {
//     ver100: SubTUSBAudioStreamingInterfaceDescriptorVer100,
//     ver200: SubTUSBAudioStreamingInterfaceDescriptorVer200,
// }

// #[repr(C, packed)]
// pub(crate) struct TUSBAudioTypeIFormatTypeDescriptor {
//     b_length: u8,
//     b_descriptor_type: u8,
//     b_descriptor_subtype: u8,
//     b_format_type: u8,
// }

// impl TUSBAudioTypeIFormatTypeDescriptor {
//     pub(crate) const USB_AUDIO_FORMAT_TYPE: u8 = 0x02;
//     pub(crate) const USB_AUDIO_FORMAT_TYPE_I: u8 = 0x01;
// }

// #[repr(C, packed)]
// pub(crate) struct SubTUSBAudioTypeIFormatTypeDescriptorVer100 {
//     b_nr_channels: u8,
//     b_subframe_size: u8,
//     b_bit_resolution: u8,
//     b_sam_freq_type: u8,
//     t_sam_freq: [[u8; 3]],
// }

// #[repr(C, packed)]
// pub(crate) struct SubTUSBAudioTypeIFormatTypeDescriptorV200 {
//     b_subslot_size: u8,
//     b_bit_resolution: u8,
// }

// pub(crate) union SubFormatTypeDescriptorVerUnion {
//     ver100: SubTUSBAudioTypeIFormatTypeDescriptorVer100,
//     ver200: SubTUSBAudioTypeIFormatTypeDescriptorV200,
// }

// // Audio class control requests
// pub(crate) const USB_AUDIO_REQ_SET_CUR: u8 = 0x01;
// pub(crate) const USB_AUDIO_REQ_RANGE: u8 = 0x02;
// pub(crate) const USB_AUDIO_REQ_GET_MIN: u8 = 0x82;
// pub(crate) const USB_AUDIO_REQ_GET_MAX: u8 = 0x83;

// // Audio class control selectors
// pub(crate) const USB_AUDIO_CS_SAM_FREQ_CONTROL: u8 = 0x01;
// pub(crate) const USB_AUDIO_FU_MUTE_CONTROL: u8 = 0x01;
// pub(crate) const USB_AUDIO_FU_VOLUME_CONTROL: u8 = 0x02;
// pub(crate) const USB_AUDIO_SU_SELECTOR_CONTROL: u8 = 0x01;
// pub(crate) const USB_AUDIO_CX_SELECTOR_CONTROL: u8 = 0x01;
