use itertools::Itertools;
use nalgebra::Vector2;
use std::sync::Arc;
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceExtensions, QueueFlags,
    },
    image::{Image, ImageUsage},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo},
    VulkanLibrary,
};
use winit::window::Window;

pub const UNIVERSALLY_REQUIRED_EXTENSIONS: DeviceExtensions = DeviceExtensions {
    khr_swapchain: true,
    ..DeviceExtensions::empty()
};

pub fn create_vulkan_instance(
    display_api_handle: Arc<Window>,
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
    let possible_canidates: Vec<_> = instance
        .enumerate_physical_devices()
        .unwrap()
        // Make sure whatever device has all of our required extensions
        .filter(|p| {
            p.supported_extensions()
                .contains(&required_extensions.union(&UNIVERSALLY_REQUIRED_EXTENSIONS))
        })
        // Grab one with a graphics queue
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.intersects(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|i| (p, i as u32))
        })
        // Sort by the device power
        .sorted_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        })
        .collect();

    assert!(
        !possible_canidates.is_empty(),
        "No suitable vulkan device found"
    );

    possible_canidates
        .iter()
        .find_map(|(p, i)| {
            if p.supported_extensions().contains(preferred_extensions) {
                Some((p.clone(), *i))
            } else {
                None
            }
        })
        // Just grab a random one if we can't find something that meets all of our requirements
        .or_else(|| possible_canidates.first().cloned())
        .map(|(p, i)| {
            let extensions = required_extensions
                .union(&UNIVERSALLY_REQUIRED_EXTENSIONS)
                .union(&p.supported_extensions().intersection(preferred_extensions));

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
        let image_format = device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

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
