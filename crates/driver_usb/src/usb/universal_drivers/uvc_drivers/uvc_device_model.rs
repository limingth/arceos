use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    vec::Vec,
};

use crate::{
    err,
    host::data_structures::MightBeInited,
    usb::descriptors::{
        desc_endpoint::Endpoint,
        desc_interface::Interface,
        desc_uvc::{
            uvc_endpoints::UVCVideoControlInterruptEndpoint,
            uvc_interfaces::{
                UVCControlInterface, UVCControlInterfaceExtensionUnit, UVCControlInterfaceHeader,
                UVCControlInterfaceInputTerminal, UVCControlInterfaceOutputTerminal,
                UVCControlInterfaceProcessingUnit, UVCInterface, UVCStreamingFormartInterface,
                UVCStreamingFrameInterface, UVCStreamingInterface, UVCVSInterfaceFormatMJPEG,
                UVCVSInterfaceInputHeader,
            },
        },
        topological_desc::TopologicalUSBDescriptorEndpoint,
        USBDescriptor,
    },
};

#[derive(Debug)]
pub struct UVCControlInterfaceModelParser {
    pub header: Option<UVCControlInterfaceHeader>,
    pub outputs: Vec<UVCControlInterfaceOutputTerminal>,
    pub inputs: Vec<UVCControlInterfaceInputTerminal>,
    pub units: Vec<Unit>,
    pub control_interrupt_endpoint: Option<(Endpoint, UVCVideoControlInterruptEndpoint)>,
}

#[derive(Debug)]
pub struct UVCControlInterfaceModel {
    pub header: UVCControlInterfaceHeader,
    pub outputs: Vec<UVCControlInterfaceOutputTerminal>,
    pub inputs: Vec<UVCControlInterfaceInputTerminal>,
    pub units: Vec<Unit>,
    pub control_interrupt_endpoint: Option<(Endpoint, UVCVideoControlInterruptEndpoint)>,
}

#[derive(Debug)]
pub enum Unit {
    ExtensionUnit(UVCControlInterfaceExtensionUnit),
    ProcessingUnit(UVCControlInterfaceProcessingUnit),
    EncodingUnit, //TODO
    SelectorUnit, //TODO
}

impl UVCControlInterfaceModelParser {
    pub fn new(
        (_, controls, interript_endpoint): &(
            Interface,
            Vec<USBDescriptor>,
            Vec<TopologicalUSBDescriptorEndpoint>,
        ),
    ) -> Self {
        let mut uvccontrol_interface_model_parser = Self {
            header: None,
            outputs: Vec::new(),
            inputs: Vec::new(),
            units: Vec::new(),
            control_interrupt_endpoint: None,
        };

        controls.iter().for_each(|c| match c {
            USBDescriptor::UVCInterface(UVCInterface::Control(control)) => match control {
                UVCControlInterface::Header(header) => {
                    uvccontrol_interface_model_parser.header = Some(header.clone())
                }
                UVCControlInterface::OutputTerminal(output) => uvccontrol_interface_model_parser
                    .outputs
                    .push(output.clone()),
                UVCControlInterface::InputTerminal(input) => {
                    uvccontrol_interface_model_parser.inputs.push(input.clone())
                }
                UVCControlInterface::ExtensionUnit(ext) => uvccontrol_interface_model_parser
                    .units
                    .push(Unit::ExtensionUnit(ext.clone())),
                UVCControlInterface::ProcessingUnit(processing) => {
                    uvccontrol_interface_model_parser
                        .units
                        .push(Unit::ProcessingUnit(processing.clone()))
                }
            },
            _ => panic!("error while parsing UVC control model!"),
        });

        if !interript_endpoint.is_empty() {
            let mut interrupt_ep = None;
            let mut interrupt_uvc_ep = None;
            interript_endpoint.iter().for_each(|e| match e {
                TopologicalUSBDescriptorEndpoint::Standard(interrupt) => {
                    interrupt_ep = Some(interrupt.clone())
                }
                TopologicalUSBDescriptorEndpoint::UNVVideoControlInterruptEndpoint(ep) => {
                    interrupt_uvc_ep = Some(ep.clone())
                }
            });
            uvccontrol_interface_model_parser.control_interrupt_endpoint =
                Some((interrupt_ep.unwrap(), interrupt_uvc_ep.unwrap()));
        };
        uvccontrol_interface_model_parser
    }

    pub fn parse(self) -> UVCControlInterfaceModel {
        UVCControlInterfaceModel {
            header: self.header.unwrap(),
            outputs: self.outputs,
            inputs: self.inputs,
            units: self.units,
            control_interrupt_endpoint: self.control_interrupt_endpoint,
        }
    }
}

/*
 * TODO:
 * StatusPacket-fetch from Interrupt endpoint, variable size, should refer page 30 to determine rest structure
*/

pub struct UVCVSInterfaceModel {
    pub interface0_stream_desca: Interface,
    pub input_header: UVCVSInterfaceInputHeader,
    pub output_header: (), //todo!
    pub formarts: Vec<(
        UVCStreamingFormartInterface,
        Vec<UVCStreamingFrameInterface>,
    )>,
    pub still_frames: Vec<UVCStreamingFrameInterface>,
    pub colorformart: Vec<UVCStreamingFormartInterface>,
}

impl UVCVSInterfaceModel {
    pub fn new(
        (interface, controls, _): &(
            Interface,
            Vec<USBDescriptor>,
            Vec<TopologicalUSBDescriptorEndpoint>,
        ),
    ) -> Self {
        let mut ret = Self {
            interface0_stream_desca: interface.clone(),
            input_header: controls
                .iter()
                .find_map(|a| match a {
                    USBDescriptor::UVCInterface(UVCInterface::Streaming(
                        UVCStreamingInterface::InputHeader(head),
                    )) => Some(head.clone()),
                    _ => None,
                })
                .unwrap(),
            output_header: (),
            formarts: controls
                .iter()
                .filter_map(|a| match a {
                    USBDescriptor::UVCInterface(UVCInterface::Streaming(any)) => Some(any),
                    _ => None,
                })
                .filter_map(UVCStreamingFormartInterface::filter_out_self)
                .map(|formart| (formart, Vec::new()))
                .collect(),
            still_frames: Vec::new(),
            colorformart: Vec::new(),
        };

        controls
            .iter()
            .filter_map(|a| match a {
                USBDescriptor::UVCInterface(UVCInterface::Streaming(any)) => Some(any),
                _ => None,
            })
            .filter_map(UVCStreamingFrameInterface::filter_out_self)
            .for_each(|any| {
                ret.formarts.iter_mut().for_each(|(interface, v)| {
                    if interface.ismatch(&any) {
                        v.push(any.clone())
                    }
                })
            });

        controls
            .iter()
            .filter_map(|a| match a {
                USBDescriptor::UVCInterface(UVCInterface::Streaming(any)) => Some(any),
                _ => None,
            })
            .filter_map(UVCStreamingFrameInterface::filter_out_still)
            .collect_into(&mut ret.still_frames);

        controls
            .iter()
            .filter_map(|a| match a {
                USBDescriptor::UVCInterface(UVCInterface::Streaming(any)) => Some(any),
                _ => None,
            })
            .filter_map(UVCStreamingFormartInterface::filter_out_color_formart)
            .collect_into(&mut ret.colorformart);

        ret
    }
}
