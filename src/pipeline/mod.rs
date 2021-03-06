// Copyright 2017 Dasein Phaos aka. Luxko
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! describes graphics and compute pipelines

// TODO: add graphic pipeline state description structure
// blockers: shader, rootsignature

use device::Device;
use format::DxgiFormat;
use winapi::{ID3D12PipelineState, ID3DBlob};
use error::WinError;
use comptr::ComPtr;
use shader::*;
use std::mem::transmute;

pub mod so;
pub mod blend;
pub mod rootsig;
pub mod rasterizer;
pub mod ds;
pub mod ia;
pub mod sampler;

pub type SampleDesc = ::swapchain::SampleDesc;

/// a pipeline state object
pub trait PipelineState {
    fn as_raw_ptr(&self) -> &ComPtr<ID3D12PipelineState>;
}

/// a graphics pipeline state object
#[derive(Clone, Debug)]
pub struct GraphicsPipelineState {
    pub(crate) ptr: ComPtr<ID3D12PipelineState>,
}

impl PipelineState for GraphicsPipelineState {
    #[inline]
    fn as_raw_ptr(&self) -> &ComPtr<ID3D12PipelineState> { 
        &self.ptr
    }
}

/// a graphics pipeline state cached blob
#[derive(Clone, Debug)]
pub struct GraphicsPipelineStateCache {
    pub(crate) ptr: ComPtr<ID3DBlob>,
}

/// a pipeline state object
#[derive(Clone, Debug)]
pub struct ComputePipelineState {
    pub(crate) ptr: ComPtr<ID3D12PipelineState>,
}

impl PipelineState for ComputePipelineState {
    #[inline]
    fn as_raw_ptr(&self) -> &ComPtr<ID3D12PipelineState> { 
        &self.ptr
    }
}

/// a pipeline state cached blob
#[derive(Clone, Debug)]
pub struct ComputePipelineStateCache {
    pub(crate) ptr: ComPtr<ID3DBlob>,
}

macro_rules! impl_cache_methods {
    ($PS: ident, $PSC: ident) => {
impl $PSC {
    #[inline]
    pub fn to_ffi_cache(&mut self) -> ::winapi::D3D12_CACHED_PIPELINE_STATE {
        unsafe {::winapi::D3D12_CACHED_PIPELINE_STATE{
            pCachedBlob: self.ptr.GetBufferPointer(),
            CachedBlobSizeInBytes: self.ptr.GetBufferSize(),
        }}
    }
}

impl $PS {
    /// get the cached blob
    #[inline]
    pub fn cached(&mut self) -> Result<$PSC, WinError> {
        unsafe {
            let mut ret = ::std::mem::uninitialized();
            let hr = self.ptr.GetCachedBlob(&mut ret);
            WinError::from_hresult_or_ok(hr, || $PSC{
                ptr: ComPtr::new(ret)
            })
        }
    }
}
    }
}

impl_cache_methods!(GraphicsPipelineState, GraphicsPipelineStateCache);
impl_cache_methods!(ComputePipelineState, ComputePipelineStateCache);

/// graphics pso builder
#[derive(Clone, Debug)]
pub struct GraphicsPipelineStateBuilder<'a> {
    pub rootsig: &'a rootsig::RootSig,
    pub vs: Option<VsShaderBytecode>,
    pub ps: Option<PsShaderBytecode>,
    pub ds: Option<DsShaderBytecode>,
    pub hs: Option<HsShaderBytecode>,
    pub gs: Option<GsShaderBytecode>,
    pub stream_output: so::DescBuilder<'a>,
    pub blend_state: blend::BlendDesc,
    pub sample_mask: u32,
    pub rasterizer_state: rasterizer::RasterizerDesc,
    pub depth_stencil_state: ds::DepthStencilDesc,
    pub input_layout: ia::InputLayoutBuilder<'a>,
    pub strip_cut_value: ia::StripCutValue,
    pub primitive_topology_type: ia::PrimitiveTopologyType,
    pub num_render_targets: u32,
    pub rtv_formats: [DxgiFormat; 8],
    pub dsv_format: DxgiFormat,
    pub sample_desc: SampleDesc,
    pub node_mask: u32,
    pub cache: Option<GraphicsPipelineStateCache>,
    pub flags: PipelineStateFlags,
}

impl<'a> GraphicsPipelineStateBuilder<'a> {
    #[inline]
    pub fn new(root_signature: &'a rootsig::RootSig) -> Self {
        GraphicsPipelineStateBuilder{
            rootsig: root_signature,
            vs: None, ps: None, ds: None, hs: None, gs: None,
            stream_output: Default::default(),
            blend_state: Default::default(),
            sample_mask: ::std::u32::MAX,
            rasterizer_state: Default::default(),
            depth_stencil_state: Default::default(),
            input_layout: Default::default(),
            strip_cut_value: Default::default(),
            primitive_topology_type: ia::PrimitiveTopologyType::TRIANGLE,
            num_render_targets: 1,
            rtv_formats: [::format::DXGI_FORMAT_UNKNOWN; 8],
            dsv_format: ::format::DXGI_FORMAT_D24_UNORM_S8_UINT,
            sample_desc: Default::default(),
            node_mask: 0,
            cache: None,
            flags: PipelineStateFlags::NONE,
        }
    }

    pub fn build(&mut self, device: &mut Device) -> Result<GraphicsPipelineState, WinError> {
        unsafe {
            let mut desc: ::winapi::D3D12_GRAPHICS_PIPELINE_STATE_DESC = ::std::mem::zeroed();
            desc.pRootSignature = self.rootsig.ptr.as_mut_ptr();
            if let Some(ref mut vs) = self.vs { desc.VS = vs.to_shader_bytecode(); }
            if let Some(ref mut ps) = self.ps { desc.PS = ps.to_shader_bytecode(); }
            if let Some(ref mut ds) = self.ds { desc.DS = ds.to_shader_bytecode(); }
            if let Some(ref mut hs) = self.hs { desc.HS = hs.to_shader_bytecode(); }
            if let Some(ref mut gs) = self.gs { desc.GS = gs.to_shader_bytecode(); }
            desc.StreamOutput = self.stream_output.build().0;
            desc.BlendState = transmute(self.blend_state);
            desc.SampleMask = self.sample_mask;
            desc.RasterizerState = transmute(self.rasterizer_state);
            desc.DepthStencilState = transmute(self.depth_stencil_state);
            desc.InputLayout.pInputElementDescs = self.input_layout.elements.as_ptr() as *const _;
            desc.InputLayout.NumElements = self.input_layout.elements.len() as u32;
            desc.IBStripCutValue = transmute(self.strip_cut_value);
            desc.PrimitiveTopologyType = transmute(self.primitive_topology_type);
            desc.NumRenderTargets = self.num_render_targets;
            desc.RTVFormats = transmute(self.rtv_formats);
            desc.DSVFormat = self.dsv_format;
            desc.SampleDesc = transmute(self.sample_desc);
            desc.NodeMask = self.node_mask;
            if let Some(ref mut pso) = self.cache { desc.CachedPSO = pso.to_ffi_cache(); }
            desc.Flags = transmute(self.flags);

            let mut ret = ::std::mem::uninitialized();
            let hr = device.ptr.CreateGraphicsPipelineState(
                &desc, & ::dxguid::IID_ID3D12PipelineState,
                &mut ret as *mut *mut _ as *mut *mut _
            );
            WinError::from_hresult_or_ok(hr, || GraphicsPipelineState{
                ptr: ComPtr::new(ret)
            })
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ComputePipelineStateBuilder<'a> {
    pub rootsig: &'a rootsig::RootSig,
    pub cs: Option<CsShaderBytecode>,
    pub node_mask: u32,
    pub cache: Option<ComputePipelineStateCache>,
    pub flags: PipelineStateFlags,
}

impl<'a> ComputePipelineStateBuilder<'a> {
    #[inline]
    pub fn new(root_signature: &'a rootsig::RootSig) -> Self {
        ComputePipelineStateBuilder{
            rootsig: root_signature,
            cs: None,
            node_mask: 0,
            cache: None,
            flags: PipelineStateFlags::NONE,
        }
    }

    pub fn build(&mut self, device: &mut Device) -> Result<ComputePipelineState, WinError> {
        unsafe {
            let mut desc: ::winapi::D3D12_COMPUTE_PIPELINE_STATE_DESC = ::std::mem::zeroed();
            desc.pRootSignature = self.rootsig.ptr.as_mut_ptr();
            if let Some(ref mut cs) = self.cs { desc.CS = cs.to_shader_bytecode(); }
            desc.NodeMask = self.node_mask;
            if let Some(ref mut pso) = self.cache { desc.CachedPSO = pso.to_ffi_cache(); }
            desc.Flags = transmute(self.flags);

            let mut ret = ::std::mem::uninitialized();
            let hr = device.ptr.CreateComputePipelineState(
                &desc, & ::dxguid::IID_ID3D12PipelineState,
                &mut ret as *mut *mut _ as *mut *mut _
            );
            WinError::from_hresult_or_ok(hr, || ComputePipelineState{
                ptr: ComPtr::new(ret)
            })
        }
    }
}

bitflags!{
    #[repr(C)]
    pub struct PipelineStateFlags: u32 {
        const NONE        = 0;
        const TOOL_DEBUG  = 0x1;
    }
}

impl Default for PipelineStateFlags{
    #[inline]
    fn default() -> Self {
        PipelineStateFlags::NONE
    }
}

bitflags!{
    /// comparison options, specifying under which circumstance the comparison passes
    #[repr(C)]
    pub struct ComparisonFunc: u32 {
        const NEVER          = 1;
        const LESS           = 2;
        const EQUAL          = 3;
        const LESS_EQUAL     = 4;
        const GREATER        = 5;
        const NOT_EQUAL      = 6;
        const GREATER_EQUAL  = 7;
        const ALWAYS         = 8;
    }
}
