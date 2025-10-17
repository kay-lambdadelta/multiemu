use multiemu_runtime::graphics::vulkan::vulkano::{
    VulkanLibrary,
    device::{
        Device, DeviceExtensions, QueueFlags,
        physical::{PhysicalDevice, PhysicalDeviceType},
    },
    format::Format,
    image::{Image, ImageUsage},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo},
};
use nalgebra::Vector2;
use std::sync::Arc;

use crate::windowing::WinitWindow;

pub const UNIVERSALLY_REQUIRED_EXTENSIONS: DeviceExtensions = DeviceExtensions {
    khr_swapchain: true,
    ..DeviceExtensions::empty()
};

pub const UNIVERSALLY_PREFERRED_EXTENSIONS: DeviceExtensions = DeviceExtensions {
    ..DeviceExtensions::empty()
};

pub fn create_vulkan_instance(
    display_api_handle: WinitWindow,
    library: Arc<VulkanLibrary>,
) -> Arc<Instance> {
    let required_extensions = Surface::required_extensions(&display_api_handle).unwrap();

    Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .unwrap()
}

pub fn select_vulkan_device(
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    preferred_extensions: &DeviceExtensions,
    required_extensions: &DeviceExtensions,
) -> Option<(Arc<PhysicalDevice>, DeviceExtensions, u32)> {
    let preferred_extensions = preferred_extensions.union(&UNIVERSALLY_PREFERRED_EXTENSIONS);
    let required_extensions = required_extensions.union(&UNIVERSALLY_REQUIRED_EXTENSIONS);

    let mut possible_candidates: Vec<_> = instance
        .enumerate_physical_devices()
        .ok()?
        .filter(|p| p.supported_extensions().contains(&required_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .find(|(i, q)| {
                    q.queue_flags.intersects(QueueFlags::GRAPHICS)
                        && p.surface_support(*i as u32, &surface).unwrap_or(false)
                })
                .map(|(i, _)| (p.clone(), i as u32))
        })
        .collect();

    possible_candidates.sort_by(|(p1, _), (p2, _)| {
        let power1 = match p1.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 5usize,
            PhysicalDeviceType::IntegratedGpu => 4,
            PhysicalDeviceType::VirtualGpu => 3,
            PhysicalDeviceType::Cpu => 2,
            PhysicalDeviceType::Other => 1,
            _ => 0,
        };

        let power2 = match p2.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 5,
            PhysicalDeviceType::IntegratedGpu => 4,
            PhysicalDeviceType::VirtualGpu => 3,
            PhysicalDeviceType::Cpu => 2,
            PhysicalDeviceType::Other => 1,
            _ => 0,
        };

        let extension_count1 = p1
            .supported_extensions()
            .intersection(&preferred_extensions)
            .count();

        let extension_count2 = p2
            .supported_extensions()
            .intersection(&preferred_extensions)
            .count();

        power2
            // Sort power in descending order
            .cmp(&power1)
            // Sort preferred extensions in descending order
            .then(extension_count2.cmp(&extension_count1))
    });

    possible_candidates.first().cloned().map(|(p, i)| {
        let extensions = required_extensions
            .union(&p.supported_extensions().intersection(&preferred_extensions));

        (p.clone(), extensions, i)
    })
}

pub fn create_vulkan_swapchain(
    device: Arc<Device>,
    surface: Arc<Surface>,
    window_dimensions: Vector2<u32>,
    vsync: bool,
) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
    let (swapchain, swapchain_images) = {
        let surface_capabilities = device
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .unwrap();
        let surface_formats = device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap();
        let image_format = surface_formats
            .iter()
            .find_map(|(format, _)| {
                if *format == Format::B8G8R8A8_SRGB || *format == Format::R8G8B8A8_SRGB {
                    Some(*format)
                } else {
                    None
                }
            })
            .unwrap_or(surface_formats[0].0);

        tracing::info!("Choosing swapchain format {:?}", image_format);

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count.max(2),
                image_format,
                image_extent: window_dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .into_iter()
                    .next()
                    .unwrap(),
                present_mode: if vsync {
                    PresentMode::Fifo
                } else {
                    PresentMode::Immediate
                },
                ..Default::default()
            },
        )
        .unwrap()
    };

    (swapchain, swapchain_images)
}
