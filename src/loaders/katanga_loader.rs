use anyhow::{bail, Context};
use ash::vk;
use wgpu::{Device, Instance, Queue};
use windows::{
    core::s,
    core::w,
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                D3D11CreateDevice, ID3D11Texture2D, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
                D3D11_TEXTURE2D_DESC,
            },
            Direct3D12::{D3D12CreateDevice, ID3D12Device, ID3D12Resource},
        },
        System::Memory::{
<<<<<<< HEAD
            MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, FILE_MAP_READ,
            MEMORY_MAPPED_VIEW_ADDRESS,
=======
            MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
            MEMORYMAPPEDVIEW_HANDLE,
>>>>>>> parent of afea657 (Better profiling automatic loader upgrade, properly handle katanga unmapping)
        },
    },
};

use crate::{
    engine::{
        formats::InternalColorFormat,
        texture::{Bound, Texture2D},
    },
    loaders::StereoMode,
    utils::external_texture::{ExternalApi, ExternalTextureInfo},
};

use super::{Loader, TextureSource};

#[derive(Default)]
pub struct KatangaLoaderContext {
    katanga_file_handle: Option<HANDLE>,
    katanga_file_mapping: Option<MEMORY_MAPPED_VIEW_ADDRESS>,
    current_address: usize,
<<<<<<< HEAD
    d3d11: Option<D3D11Context>,
    d3d12: Option<D3D12Context>,
}

impl KatangaLoaderContext {
    fn unmap(&mut self) {
        if let Some(file_mapping) = self.katanga_file_mapping.take() {
            if let Err(err) = unsafe { UnmapViewOfFile(file_mapping) } {
                log::error!("Failed to unmap file view: {:?}", err);
            }
        }

        if let Some(katanga_handle) = self.katanga_file_handle.take() {
            if !katanga_handle.is_invalid() {
                if let Err(err) = unsafe { CloseHandle(katanga_handle) } {
                    log::error!("Failed to close file mapping: {:?}", err);
                }
            }
        }
    }

    fn map_katanga_file(&mut self) -> anyhow::Result<()> {
        if self.katanga_file_handle.is_some()
            && !self.katanga_file_handle.as_ref().unwrap().is_invalid()
            && self.katanga_file_mapping.is_some()
        {
            return Ok(());
        }

        self.unmap();

        self.katanga_file_handle = match unsafe {
            OpenFileMappingA(FILE_MAP_READ.0, false, s!("Local\\KatangaMappedFile"))
        } {
            Ok(handle) => Some(handle),
            Err(_) => {
                self.unmap();
                bail!("Cannot open file mapping!")
            }
        };
        log::trace!("Handle: {:?}", self.katanga_file_handle);

        self.katanga_file_mapping = Some(unsafe {
            MapViewOfFile(
                self.katanga_file_handle.unwrap(),
                FILE_MAP_READ,
                0,
                0,
                std::mem::size_of::<usize>(),
            )
        });

        if self.katanga_file_mapping.unwrap().Value.is_null() {
            self.unmap();
            bail!("Cannot map file view!");
        }

        Ok(())
    }
}

impl Default for KatangaLoaderContext {
    fn default() -> Self {
        Self {
            d3d11: D3D11Context::new().ok(),
            d3d12: D3D12Context::new().ok(),
            katanga_file_handle: None,
            katanga_file_mapping: None,
            current_address: 0,
        }
    }
}

struct D3D11Context {
    device: ID3D11Device,
    _device_context: ID3D11DeviceContext,
}

impl D3D11Context {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn new() -> anyhow::Result<Self> {
        let mut device = None;
        let mut device_context = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut device_context),
            )?
        };

        Ok(Self {
            device: device.context("Failed to create D3D11 device")?,
            _device_context: device_context.context("Failed to create D3D11 device context")?,
        })
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn get_d3d11_texture_info(&self, handle: HANDLE) -> anyhow::Result<ExternalTextureInfo> {
        let mut d3d11_texture: Option<ID3D11Texture2D> = None;
        unsafe { self.device.OpenSharedResource(handle, &mut d3d11_texture) }?;
        let d3d11_texture = d3d11_texture.context("Failed to open shared DX11 texture")?;
        let mut texture_desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { d3d11_texture.GetDesc(&mut texture_desc) };

        let format: InternalColorFormat = texture_desc.Format.try_into()?;
        log::info!("Got texture from DX11 with format {:?}", format);

        Ok(ExternalTextureInfo {
            external_api: ExternalApi::D3D11,
            width: texture_desc.Width,
            height: texture_desc.Height,
            array_size: texture_desc.ArraySize,
            sample_count: texture_desc.SampleDesc.Count,
            mip_levels: texture_desc.MipLevels,
            format,
            actual_handle: handle.0 as usize,
        })
    }
}

struct D3D12Context {
    device: ID3D12Device,
}

impl D3D12Context {
    pub fn new() -> anyhow::Result<Self> {
        let mut d3d12_device: Option<ID3D12Device> = None;
        unsafe {
            D3D12CreateDevice(
                None,
                windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0,
                &mut d3d12_device,
            )
        }?;

        let d3d12_device = d3d12_device.context("DX12 device not initialized")?;

        Ok(Self {
            device: d3d12_device,
        })
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn get_d3d12_named_texture_info(
        &self,
        texture_name: PCWSTR,
    ) -> anyhow::Result<ExternalTextureInfo> {
        let named_handle = unsafe {
            self.device.OpenSharedHandleByName(texture_name, 0x10000000) //GENERIC_ALL
        }?;

        let mut d3d12_texture: Option<ID3D12Resource> = None;
        unsafe {
            self.device
                .OpenSharedHandle(named_handle, &mut d3d12_texture)
        }?;
        let d3d12_texture = d3d12_texture.context("Failed to open shared DX12 texture")?;

        let tex_info = unsafe { d3d12_texture.GetDesc() };

        let format: InternalColorFormat = tex_info.Format.try_into()?;
        log::info!("Got texture from DX12 with format {:?}", format);

        Ok(ExternalTextureInfo {
            external_api: ExternalApi::D3D12,
            width: tex_info.Width as u32,
            height: tex_info.Height,
            array_size: tex_info.DepthOrArraySize as u32,
            sample_count: tex_info.SampleDesc.Count,
            mip_levels: tex_info.MipLevels as u32,
            format,
            actual_handle: named_handle.0 as usize,
        })
    }
=======
>>>>>>> parent of afea657 (Better profiling automatic loader upgrade, properly handle katanga unmapping)
}

impl Loader for KatangaLoaderContext {
    #[cfg_attr(feature = "profiling", profiling::function)]
<<<<<<< HEAD
    fn load(
        &mut self,
        _instance: &Instance,
        device: &Device,
        _queue: &Queue,
    ) -> anyhow::Result<TextureSource> {
        self.map_katanga_file()?;
=======
    fn load(&mut self, _instance: &Instance, device: &Device) -> anyhow::Result<TextureSource> {
        self.katanga_file_handle = unsafe {
            OpenFileMappingA(FILE_MAP_ALL_ACCESS.0, false, s!("Local\\KatangaMappedFile"))?
        };
        log::info!("Handle: {:?}", self.katanga_file_handle);

        self.katanga_file_mapping = unsafe {
            MapViewOfFile(
                self.katanga_file_handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                std::mem::size_of::<usize>(),
            )?
        };

        if self.katanga_file_mapping.is_invalid() {
            bail!("Cannot map file!");
        }
>>>>>>> parent of afea657 (Better profiling automatic loader upgrade, properly handle katanga unmapping)

        let address = unsafe { *(self.katanga_file_mapping.as_ref().unwrap().Value as *mut usize) };
        self.current_address = address;
        let tex_handle = self.current_address as vk::HANDLE;
        log::info!("{:#01x}", tex_handle as usize);

        let tex_info = get_d3d11_texture_info(HANDLE(tex_handle as isize)).or_else(|err| {
            log::warn!("Not a D3D11Texture {}", err);
            get_d3d12_texture_info().map_err(|err| {
                log::warn!("Not a D3D12Texture {}", err);
                err
            })
        })?;

        if tex_info.actual_handle != self.current_address {
            log::info!("Actual Handle: {:?}", self.katanga_file_handle);
        }

        let screen_format = tex_info.format;
        let screen_norm_format = screen_format.to_norm();
        let view_formats = if screen_norm_format != screen_format {
            Some(screen_norm_format)
        } else {
            None
        };

        let internal_texture =
            tex_info.map_as_wgpu_texture("KatangaStream", device, view_formats)?;

        Ok(TextureSource {
            texture: internal_texture,
            width: tex_info.width,
            height: tex_info.height,
            stereo_mode: Some(StereoMode::FullSbs),
        })
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn is_invalid(&self) -> bool {
<<<<<<< HEAD
        if self.katanga_file_mapping.is_none()
            || self.katanga_file_handle.is_none()
            || self.katanga_file_handle.as_ref().unwrap().is_invalid()
            || self.katanga_file_mapping.as_ref().unwrap().Value.is_null()
        {
            return true;
        }

        let address = unsafe { *(self.katanga_file_mapping.as_ref().unwrap().Value as *mut usize) };
=======
        let address = unsafe { *(self.katanga_file_mapping.0 as *mut usize) };
>>>>>>> parent of afea657 (Better profiling automatic loader upgrade, properly handle katanga unmapping)
        self.current_address != address
    }

    // No update needed for Katanga
    #[cfg_attr(feature = "profiling", profiling::function)]
    fn update(
        &mut self,
        _instance: &Instance,
        _device: &Device,
        _queue: &Queue,
        _texture: &Texture2D<Bound>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn encode_pre_pass(
        &self,
        _encoder: &mut wgpu::CommandEncoder,
        _texture: &Texture2D<Bound>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Drop for KatangaLoaderContext {
    #[cfg_attr(feature = "profiling", profiling::function)]
    fn drop(&mut self) {
        log::info!("Dropping KatangaLoaderContext");

        if !self.katanga_file_mapping.is_invalid()
            && unsafe { bool::from(UnmapViewOfFile(self.katanga_file_mapping)) }
        {
            log::info!("Unmapped file!");
        }

        if !self.katanga_file_handle.is_invalid()
            && unsafe { bool::from(CloseHandle(self.katanga_file_handle)) }
        {
            log::info!("Closed handle!");
        }
    }
}
<<<<<<< HEAD
=======

struct ExternalTextureInfo {
    external_api: ExternalApi,
    width: u32,
    height: u32,
    array_size: u32,
    sample_count: u32,
    mip_levels: u32,
    format: TextureFormat,
    actual_handle: usize,
}

enum ExternalApi {
    D3D11,
    D3D12,
}

#[cfg_attr(feature = "profiling", profiling::function)]
fn get_d3d11_texture_info(handle: HANDLE) -> anyhow::Result<ExternalTextureInfo> {
    let mut d3d11_device = None;
    let mut d3d11_device_context = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut d3d11_device),
            None,
            Some(&mut d3d11_device_context),
        )?
    };
    let mut d3d11_texture: Option<ID3D11Texture2D> = None;
    unsafe {
        d3d11_device
            .as_ref()
            .context("DX11 device not initialized")?
            .OpenSharedResource(handle, &mut d3d11_texture)
    }?;
    let d3d11_texture = d3d11_texture.context("Failed to open shared DX11 texture")?;
    let mut texture_desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { d3d11_texture.GetDesc(&mut texture_desc) };

    log::info!(
        "Got texture from DX11 with format {:?}",
        texture_desc.Format
    );

    Ok(ExternalTextureInfo {
        external_api: ExternalApi::D3D11,
        width: texture_desc.Width,
        height: texture_desc.Height,
        array_size: texture_desc.ArraySize,
        sample_count: texture_desc.SampleDesc.Count,
        mip_levels: texture_desc.MipLevels,
        format: unmap_texture_format(texture_desc.Format),
        actual_handle: handle.0 as usize,
    })
}

#[cfg_attr(feature = "profiling", profiling::function)]
fn get_d3d12_texture_info() -> anyhow::Result<ExternalTextureInfo> {
    let mut d3d12_device: Option<ID3D12Device> = None;
    unsafe {
        D3D12CreateDevice(
            None,
            windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0,
            &mut d3d12_device,
        )
    }?;

    let d3d12_device = d3d12_device
        .as_ref()
        .context("DX12 device not initialized")?;

    let named_handle = unsafe {
        d3d12_device.OpenSharedHandleByName(w!("DX12VRStream"), 0x10000000) //GENERIC_ALL
    }?;

    let mut d3d12_texture: Option<ID3D12Resource> = None;
    unsafe { d3d12_device.OpenSharedHandle(named_handle, &mut d3d12_texture) }?;
    let d3d12_texture = d3d12_texture.context("Failed to open shared DX12 texture")?;

    let tex_info = unsafe { d3d12_texture.GetDesc() };

    log::info!("Got texture from DX12 with format {:?}", tex_info.Format);

    Ok(ExternalTextureInfo {
        external_api: ExternalApi::D3D12,
        width: tex_info.Width as u32,
        height: tex_info.Height,
        array_size: tex_info.DepthOrArraySize as u32,
        sample_count: tex_info.SampleDesc.Count,
        mip_levels: tex_info.MipLevels as u32,
        format: unmap_texture_format(tex_info.Format),
        actual_handle: named_handle.0 as usize,
    })
}
>>>>>>> parent of afea657 (Better profiling automatic loader upgrade, properly handle katanga unmapping)
