embassy_hal_internal::peripherals_definition!(
    PA0,
    PA1,
    PA2,
    PA3,
    PA4,
    PA5,
    PA6,
    PA7,
    PA8,
    PA9,
    PA10,
    PA11,
    PA12,
    PA13,
    PA14,
    PA15,
    PB0,
    PB1,
    PB2,
    PB3,
    PB4,
    PB5,
    PB6,
    PB7,
    PB8,
    PB9,
    PB10,
    PB11,
    PB12,
    PB13,
    PB14,
    PB15,
    PC4,
    PC6,
    PC10,
    PC11,
    PC13,
    PC14,
    PC15,
    PF0,
    PF1,
    PG10,
    ADC1,
    ADC12_COMMON,
    ADC2,
    COMP1,
    COMP2,
    COMP3,
    COMP4,
    CORDIC,
    CRC,
    CRS,
    DAC1,
    DAC3,
    DBGMCU,
    DMA1,
    DMA2,
    DMAMUX1,
    FDCAN1,
    FDCANRAM1,
    FLASH,
    FMAC,
    I2C1,
    I2C2,
    I2C3,
    IWDG,
    LPTIM1,
    LPUART1,
    OPAMP1,
    OPAMP2,
    OPAMP3,
    PWR,
    MCO,
    RCC,
    RNG,
    RTC,
    SAI1,
    SPI1,
    SPI2,
    SPI3,
    SYSCFG,
    TAMP,
    TIM1,
    TIM15,
    TIM16,
    TIM17,
    TIM2,
    TIM3,
    TIM4,
    TIM6,
    TIM7,
    TIM8,
    UART4,
    UCPD1,
    UID,
    USART1,
    USART2,
    USART3,
    USB,
    USBRAM,
    VREFBUF,
    WWDG,
    EXTI0,
    EXTI1,
    EXTI2,
    EXTI3,
    EXTI4,
    EXTI5,
    EXTI6,
    EXTI7,
    EXTI8,
    EXTI9,
    EXTI10,
    EXTI11,
    EXTI12,
    EXTI13,
    EXTI14,
    EXTI15,
    DMA1_CH1,
    DMA1_CH2,
    DMA1_CH3,
    DMA1_CH4,
    DMA1_CH5,
    DMA1_CH6,
    DMA2_CH1,
    DMA2_CH2,
    DMA2_CH3,
    DMA2_CH4,
    DMA2_CH5,
    DMA2_CH6
);
embassy_hal_internal::peripherals_struct!(
    PA0,
    PA1,
    PA2,
    PA3,
    PA4,
    PA5,
    PA6,
    PA7,
    PA8,
    PA9,
    PA10,
    PA11,
    PA12,
    PA13,
    PA14,
    PA15,
    PB0,
    PB1,
    PB2,
    PB3,
    PB4,
    PB5,
    PB6,
    PB7,
    PB8,
    PB9,
    PB10,
    PB11,
    PB12,
    PB13,
    PB14,
    PB15,
    PC4,
    PC6,
    PC10,
    PC11,
    PC13,
    PC14,
    PC15,
    PF0,
    PF1,
    PG10,
    ADC1,
    ADC12_COMMON,
    ADC2,
    COMP1,
    COMP2,
    COMP3,
    COMP4,
    CORDIC,
    CRC,
    CRS,
    DAC1,
    DAC3,
    DBGMCU,
    DMA1,
    DMA2,
    DMAMUX1,
    FDCAN1,
    FDCANRAM1,
    FLASH,
    FMAC,
    I2C1,
    I2C2,
    I2C3,
    IWDG,
    LPTIM1,
    LPUART1,
    OPAMP1,
    OPAMP2,
    OPAMP3,
    PWR,
    MCO,
    RCC,
    RNG,
    RTC,
    SAI1,
    SPI1,
    SPI2,
    SPI3,
    SYSCFG,
    TAMP,
    TIM1,
    TIM16,
    TIM17,
    TIM2,
    TIM3,
    TIM4,
    TIM6,
    TIM7,
    TIM8,
    UART4,
    UCPD1,
    UID,
    USART1,
    USART2,
    USART3,
    USB,
    USBRAM,
    VREFBUF,
    WWDG,
    EXTI0,
    EXTI1,
    EXTI2,
    EXTI3,
    EXTI4,
    EXTI5,
    EXTI6,
    EXTI7,
    EXTI8,
    EXTI9,
    EXTI10,
    EXTI11,
    EXTI12,
    EXTI13,
    EXTI14,
    EXTI15,
    DMA1_CH1,
    DMA1_CH2,
    DMA1_CH3,
    DMA1_CH4,
    DMA1_CH5,
    DMA1_CH6,
    DMA2_CH1,
    DMA2_CH2,
    DMA2_CH3,
    DMA2_CH4,
    DMA2_CH5,
    DMA2_CH6
);
embassy_hal_internal::interrupt_mod!(
    WWDG,
    PVD_PVM,
    RTC_TAMP_LSECSS,
    RTC_WKUP,
    FLASH,
    RCC,
    EXTI0,
    EXTI1,
    EXTI2,
    EXTI3,
    EXTI4,
    DMA1_CHANNEL1,
    DMA1_CHANNEL2,
    DMA1_CHANNEL3,
    DMA1_CHANNEL4,
    DMA1_CHANNEL5,
    DMA1_CHANNEL6,
    ADC1_2,
    USB_HP,
    USB_LP,
    FDCAN1_IT0,
    FDCAN1_IT1,
    EXTI9_5,
    TIM1_BRK_TIM15,
    TIM1_UP_TIM16,
    TIM1_TRG_COM_TIM17,
    TIM1_CC,
    TIM2,
    TIM3,
    TIM4,
    I2C1_EV,
    I2C1_ER,
    I2C2_EV,
    I2C2_ER,
    SPI1,
    SPI2,
    USART1,
    USART2,
    USART3,
    EXTI15_10,
    RTC_ALARM,
    USBWAKEUP,
    TIM8_BRK,
    TIM8_UP,
    TIM8_TRG_COM,
    TIM8_CC,
    LPTIM1,
    SPI3,
    UART4,
    TIM6_DAC,
    TIM7,
    DMA2_CHANNEL1,
    DMA2_CHANNEL2,
    DMA2_CHANNEL3,
    DMA2_CHANNEL4,
    DMA2_CHANNEL5,
    UCPD1,
    COMP1_2_3,
    COMP4,
    CRS,
    SAI1,
    FPU,
    RNG,
    LPUART1,
    I2C3_EV,
    I2C3_ER,
    DMAMUX_OVR,
    DMA2_CHANNEL6,
    CORDIC,
    FMAC,
);
pub const MAX_ERASE_SIZE: usize = 2048u32 as usize;
pub mod flash_regions {
    pub const BANK1_REGION: crate::flash::FlashRegion = crate::flash::FlashRegion {
        bank: crate::flash::FlashBank::Bank1,
        base: 134217728u32,
        size: 131072u32,
        erase_size: 2048u32,
        write_size: 8u32,
        erase_value: 255u8,
        _ensure_internal: (),
    };
    #[cfg(flash)]
    pub struct Bank1Region<'d, MODE = crate::flash::Async>(
        pub &'static crate::flash::FlashRegion,
        pub(crate) embassy_hal_internal::Peri<'d, crate::peripherals::FLASH>,
        pub(crate) core::marker::PhantomData<MODE>,
    );
    #[cfg(flash)]
    pub struct FlashLayout<'d, MODE = crate::flash::Async> {
        pub bank1_region: Bank1Region<'d, MODE>,
        _mode: core::marker::PhantomData<MODE>,
    }
    #[cfg(flash)]
    impl<'d, MODE> FlashLayout<'d, MODE> {
        pub(crate) fn new(p: embassy_hal_internal::Peri<'d, crate::peripherals::FLASH>) -> Self {
            Self {
                bank1_region: Bank1Region(
                    &BANK1_REGION,
                    unsafe { p.clone_unchecked() },
                    core::marker::PhantomData,
                ),
                _mode: core::marker::PhantomData,
            }
        }
    }
    pub const FLASH_REGIONS: [&crate::flash::FlashRegion; 1usize] = [&BANK1_REGION];
}
impl crate::rcc::SealedRccPeripheral for peripherals::ADC1 {
    fn frequency() -> crate::time::Hertz {
        match crate::pac::RCC.ccipr().read().adc12sel() {
            crate::pac::rcc::vals::Adcsel::PLL1_P => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . pll1_p . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "ADC1" , "pll1_p")
            },
            crate::pac::rcc::vals::Adcsel::SYS => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . sys . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "ADC1" , "sys")
            },
            #[allow(unreachable_patterns)]
            _ => panic!(
                "attempted to use peripheral '{}' but its clock mux is not set to a valid \
                         clock. Change 'config.rcc.mux' to another clock.",
                "ADC1"
            ),
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "ADC1" , "hclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((11u8, 13u8)),
            (19u8, 13u8),
            Some(0u8),
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::ADC1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::ADC2 {
    fn frequency() -> crate::time::Hertz {
        match crate::pac::RCC.ccipr().read().adc12sel() {
            crate::pac::rcc::vals::Adcsel::PLL1_P => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . pll1_p . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "ADC2" , "pll1_p")
            },
            crate::pac::rcc::vals::Adcsel::SYS => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . sys . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "ADC2" , "sys")
            },
            #[allow(unreachable_patterns)]
            _ => panic!(
                "attempted to use peripheral '{}' but its clock mux is not set to a valid \
                         clock. Change 'config.rcc.mux' to another clock.",
                "ADC2"
            ),
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "ADC2" , "hclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((11u8, 13u8)),
            (19u8, 13u8),
            Some(0u8),
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::ADC2 {}
impl crate::rcc::SealedRccPeripheral for peripherals::CORDIC {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "CORDIC" , "hclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "CORDIC" , "hclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((10u8, 3u8)),
            (18u8, 3u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::CORDIC {}
impl crate::rcc::SealedRccPeripheral for peripherals::CRC {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "CRC" , "hclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "CRC" , "hclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((10u8, 12u8)),
            (18u8, 12u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::CRC {}
impl crate::rcc::SealedRccPeripheral for peripherals::CRS {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "CRS" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "CRS" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 8u8)),
            (22u8, 8u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::CRS {}
impl crate::rcc::SealedRccPeripheral for peripherals::DAC1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DAC1" , "hclk2")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DAC1" , "hclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((11u8, 16u8)),
            (19u8, 16u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::DAC1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::DAC3 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DAC3" , "hclk2")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DAC3" , "hclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((11u8, 18u8)),
            (19u8, 18u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::DAC3 {}
impl crate::rcc::SealedRccPeripheral for peripherals::DMA1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DMA1" , "hclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DMA1" , "hclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((10u8, 0u8)),
            (18u8, 0u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::DMA1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::DMA2 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DMA2" , "hclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DMA2" , "hclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((10u8, 1u8)),
            (18u8, 1u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::DMA2 {}
impl crate::rcc::SealedRccPeripheral for peripherals::DMAMUX1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DMAMUX1" , "hclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "DMAMUX1" , "hclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((10u8, 2u8)),
            (18u8, 2u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::DMAMUX1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::FDCAN1 {
    fn frequency() -> crate::time::Hertz {
        match crate::pac::RCC.ccipr().read().fdcansel() {
            crate::pac::rcc::vals::Fdcansel::HSE => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . hse . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FDCAN1" , "hse")
            },
            crate::pac::rcc::vals::Fdcansel::PLL1_Q => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . pll1_q . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FDCAN1" , "pll1_q")
            },
            crate::pac::rcc::vals::Fdcansel::PCLK1 => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FDCAN1" , "pclk1")
            },
            #[allow(unreachable_patterns)]
            _ => panic!(
                "attempted to use peripheral '{}' but its clock mux is not set to a valid \
                         clock. Change 'config.rcc.mux' to another clock.",
                "FDCAN1"
            ),
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FDCAN1" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 25u8)),
            (22u8, 25u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::FDCAN1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::FLASH {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FLASH" , "hclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FLASH" , "hclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((10u8, 8u8)),
            (18u8, 8u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::FLASH {}
impl crate::rcc::SealedRccPeripheral for peripherals::FMAC {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FMAC" , "hclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "FMAC" , "hclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((10u8, 4u8)),
            (18u8, 4u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::FMAC {}
impl crate::rcc::SealedRccPeripheral for peripherals::I2C1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "I2C1" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "I2C1" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 21u8)),
            (22u8, 21u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::I2C1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::I2C2 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "I2C2" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "I2C2" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 22u8)),
            (22u8, 22u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::I2C2 {}
impl crate::rcc::SealedRccPeripheral for peripherals::I2C3 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "I2C3" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "I2C3" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 30u8)),
            (22u8, 30u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::I2C3 {}
impl crate::rcc::SealedRccPeripheral for peripherals::LPTIM1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "LPTIM1" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "LPTIM1" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 31u8)),
            (22u8, 31u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop2,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::LPTIM1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::LPUART1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "LPUART1" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "LPUART1" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((15u8, 0u8)),
            (23u8, 0u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop2,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::LPUART1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::PWR {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "PWR" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "PWR" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 28u8)),
            (22u8, 28u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::PWR {}
impl crate::rcc::SealedRccPeripheral for peripherals::RNG {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "RNG" , "hclk2")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . hclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "RNG" , "hclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((11u8, 26u8)),
            (19u8, 26u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::RNG {}
impl crate::rcc::SealedRccPeripheral for peripherals::RTC {
    fn frequency() -> crate::time::Hertz {
        match crate::pac::RCC.bdcr().read().rtcsel() {
            crate::pac::rcc::vals::Rtcsel::LSE => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . lse . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "RTC" , "lse")
            },
            crate::pac::rcc::vals::Rtcsel::LSI => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . lsi . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "RTC" , "lsi")
            },
            crate::pac::rcc::vals::Rtcsel::HSE_DIV_32 => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . hse . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "RTC" , "hse") / 32u32
            },
            #[allow(unreachable_patterns)]
            _ => panic!(
                "attempted to use peripheral '{}' but its clock mux is not set to a valid \
                         clock. Change 'config.rcc.mux' to another clock.",
                "RTC"
            ),
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "RTC" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            None,
            (22u8, 10u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Standby,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::RTC {}
impl crate::rcc::SealedRccPeripheral for peripherals::SAI1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SAI1" , "pclk2")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SAI1" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 21u8)),
            (24u8, 21u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::SAI1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::SPI1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SPI1" , "pclk2")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SPI1" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 12u8)),
            (24u8, 12u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::SPI1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::SPI2 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SPI2" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SPI2" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 14u8)),
            (22u8, 14u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::SPI2 {}
impl crate::rcc::SealedRccPeripheral for peripherals::SPI3 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SPI3" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SPI3" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 15u8)),
            (22u8, 15u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::SPI3 {}
impl crate::rcc::SealedRccPeripheral for peripherals::SYSCFG {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SYSCFG" , "pclk2")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "SYSCFG" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 0u8)),
            (24u8, 0u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::SYSCFG {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM1" , "pclk2_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM1" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 11u8)),
            (24u8, 11u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM15 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM15" , "pclk2_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM15" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 16u8)),
            (24u8, 16u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM15 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM16 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM16" , "pclk2_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM16" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 17u8)),
            (24u8, 17u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM16 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM17 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM17" , "pclk2_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM17" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 18u8)),
            (24u8, 18u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM17 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM2 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM2" , "pclk1_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM2" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 0u8)),
            (22u8, 0u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM2 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM3 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM3" , "pclk1_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM3" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 1u8)),
            (22u8, 1u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM3 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM4 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM4" , "pclk1_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM4" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 2u8)),
            (22u8, 2u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM4 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM6 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM6" , "pclk1_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM6" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 4u8)),
            (22u8, 4u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM6 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM7 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM7" , "pclk1_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM7" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 5u8)),
            (22u8, 5u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM7 {}
impl crate::rcc::SealedRccPeripheral for peripherals::TIM8 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2_tim . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM8" , "pclk2_tim")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "TIM8" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 13u8)),
            (24u8, 13u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::TIM8 {}
impl crate::rcc::SealedRccPeripheral for peripherals::UART4 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "UART4" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "UART4" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 19u8)),
            (22u8, 19u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::UART4 {}
impl crate::rcc::SealedRccPeripheral for peripherals::UCPD1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "UCPD1" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "UCPD1" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((15u8, 8u8)),
            (23u8, 8u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::UCPD1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::USART1 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USART1" , "pclk2")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk2 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USART1" , "pclk2")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((16u8, 14u8)),
            (24u8, 14u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::USART1 {}
impl crate::rcc::SealedRccPeripheral for peripherals::USART2 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USART2" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USART2" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 17u8)),
            (22u8, 17u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::USART2 {}
impl crate::rcc::SealedRccPeripheral for peripherals::USART3 {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USART3" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USART3" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 18u8)),
            (22u8, 18u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::USART3 {}
impl crate::rcc::SealedRccPeripheral for peripherals::USB {
    fn frequency() -> crate::time::Hertz {
        match crate::pac::RCC.ccipr().read().clk48sel() {
            crate::pac::rcc::vals::Clk48sel::HSI48 => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . hsi48 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USB" , "hsi48")
            },
            crate::pac::rcc::vals::Clk48sel::PLL1_Q => unsafe {
                unwrap ! (crate :: rcc :: get_freqs () . pll1_q . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USB" , "pll1_q")
            },
            #[allow(unreachable_patterns)]
            _ => panic!(
                "attempted to use peripheral '{}' but its clock mux is not set to a valid \
                         clock. Change 'config.rcc.mux' to another clock.",
                "USB"
            ),
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "USB" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            Some((14u8, 23u8)),
            (22u8, 23u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::USB {}
impl crate::rcc::SealedRccPeripheral for peripherals::WWDG {
    fn frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "WWDG" , "pclk1")
        }
    }
    fn bus_frequency() -> crate::time::Hertz {
        unsafe {
            unwrap ! (crate :: rcc :: get_freqs () . pclk1 . to_hertz () , "peripheral '{}' is configured to use the '{}' clock, which is not running. \
                    Either enable it in 'config.rcc' or change 'config.rcc.mux' to use another clock" , "WWDG" , "pclk1")
        }
    }
    const RCC_INFO: crate::rcc::RccInfo = unsafe {
        crate::rcc::RccInfo::new(
            None,
            (22u8, 11u8),
            None,
            #[cfg(feature = "low-power")]
            crate::rcc::StopMode::Stop1,
        )
    };
}
impl crate::rcc::RccPeripheral for peripherals::WWDG {}
pub(crate) static mut REFCOUNTS: [u8; 1usize] = [0u8];
pub mod mux {
    pub use crate::pac::rcc::vals::Adcsel;
    pub use crate::pac::rcc::vals::Clk48sel;
    pub use crate::pac::rcc::vals::Fdcansel;
    pub use crate::pac::rcc::vals::Rtcsel;
    #[derive(Clone, Copy)]
    #[non_exhaustive]
    pub struct ClockMux {
        pub rtcsel: Rtcsel,
        pub adc12sel: Adcsel,
        pub clk48sel: Clk48sel,
        pub fdcansel: Fdcansel,
    }
    impl ClockMux {
        pub(crate) const fn default() -> Self {
            unsafe { ::core::mem::zeroed() }
        }
    }
    impl Default for ClockMux {
        fn default() -> Self {
            Self::default()
        }
    }
    impl ClockMux {
        pub(crate) fn init(&self) {
            crate::pac::RCC.bdcr().modify(|w| {
                w.set_rtcsel(self.rtcsel);
            });
            crate::pac::RCC.ccipr().modify(|w| {
                w.set_adc12sel(self.adc12sel);
                w.set_clk48sel(self.clk48sel);
                w.set_fdcansel(self.fdcansel);
            });
        }
    }
}
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct Clocks {
    pub hclk1: crate::time::MaybeHertz,
    pub hclk2: crate::time::MaybeHertz,
    pub hse: crate::time::MaybeHertz,
    pub hsi48: crate::time::MaybeHertz,
    pub lse: crate::time::MaybeHertz,
    pub lsi: crate::time::MaybeHertz,
    pub pclk1: crate::time::MaybeHertz,
    pub pclk1_tim: crate::time::MaybeHertz,
    pub pclk2: crate::time::MaybeHertz,
    pub pclk2_tim: crate::time::MaybeHertz,
    pub pll1_p: crate::time::MaybeHertz,
    pub pll1_q: crate::time::MaybeHertz,
    pub rtc: crate::time::MaybeHertz,
    pub sys: crate::time::MaybeHertz,
}
pub unsafe fn init_dma() {}
pub unsafe fn init_bdma() {
    crate::pac::RCC.ahb1enr().modify(|w| w.set_dma1en(true));
    crate::pac::RCC.ahb1enr().modify(|w| w.set_dma2en(true));
}
pub unsafe fn init_dmamux() {
    crate::pac::RCC.ahb1enr().modify(|w| w.set_dmamux1en(true));
}
pub unsafe fn init_gpdma() {}
pub unsafe fn init_gpio() {
    crate::pac::RCC.ahb2enr().modify(|w| w.set_gpioaen(true));
    crate::pac::RCC.ahb2enr().modify(|w| w.set_gpioben(true));
    crate::pac::RCC.ahb2enr().modify(|w| w.set_gpiocen(true));
    crate::pac::RCC.ahb2enr().modify(|w| w.set_gpioden(true));
    crate::pac::RCC.ahb2enr().modify(|w| w.set_gpioeen(true));
    crate::pac::RCC.ahb2enr().modify(|w| w.set_gpiofen(true));
    crate::pac::RCC.ahb2enr().modify(|w| w.set_gpiogen(true));
}
impl_adc_pin!(ADC1, PA0, 1u8);
impl_adc_pin!(ADC1, PA1, 2u8);
impl_adc_pin!(ADC1, PA2, 3u8);
impl_adc_pin!(ADC1, PA3, 4u8);
impl_adc_pin!(ADC1, PB0, 15u8);
impl_adc_pin!(ADC1, PB1, 12u8);
impl_adc_pin!(ADC1, PB11, 14u8);
impl_adc_pin!(ADC1, PB12, 11u8);
impl_adc_pin!(ADC1, PB14, 5u8);
impl_adc_pin!(ADC1, PF0, 10u8);
impl_adc_pin!(ADC2, PA0, 1u8);
impl_adc_pin!(ADC2, PA1, 2u8);
impl_adc_pin!(ADC2, PA4, 17u8);
impl_adc_pin!(ADC2, PA5, 13u8);
impl_adc_pin!(ADC2, PA6, 3u8);
impl_adc_pin!(ADC2, PA7, 4u8);
impl_adc_pin!(ADC2, PB11, 14u8);
impl_adc_pin!(ADC2, PB15, 15u8);
impl_adc_pin!(ADC2, PB2, 12u8);
impl_adc_pin!(ADC2, PC4, 5u8);
impl_adc_pin!(ADC2, PF1, 10u8);
pin_trait_impl!(crate::dac::DacPin<Ch1>, DAC1, PA4, 0u8);
pin_trait_impl!(crate::dac::DacPin<Ch2>, DAC1, PA5, 0u8);
pin_trait_impl!(crate::can::RxPin, FDCAN1, PA11, 9u8);
pin_trait_impl!(crate::can::TxPin, FDCAN1, PA12, 9u8);
pin_trait_impl!(crate::can::RxPin, FDCAN1, PB8, 9u8);
pin_trait_impl!(crate::can::TxPin, FDCAN1, PB9, 9u8);
pin_trait_impl!(crate::i2c::SclPin, I2C1, PA13, 4u8);
pin_trait_impl!(crate::i2c::SdaPin, I2C1, PA14, 4u8);
pin_trait_impl!(crate::i2c::SclPin, I2C1, PA15, 4u8);
pin_trait_impl!(crate::i2c::SdaPin, I2C1, PB7, 4u8);
pin_trait_impl!(crate::i2c::SclPin, I2C1, PB8, 4u8);
pin_trait_impl!(crate::i2c::SdaPin, I2C1, PB9, 4u8);
pin_trait_impl!(crate::i2c::SdaPin, I2C2, PA8, 4u8);
pin_trait_impl!(crate::i2c::SclPin, I2C2, PA9, 4u8);
pin_trait_impl!(crate::i2c::SclPin, I2C2, PC4, 4u8);
pin_trait_impl!(crate::i2c::SdaPin, I2C2, PF0, 4u8);
pin_trait_impl!(crate::i2c::SclPin, I2C3, PA8, 2u8);
pin_trait_impl!(crate::i2c::SdaPin, I2C3, PB5, 8u8);
pin_trait_impl!(crate::i2c::SdaPin, I2C3, PC11, 8u8);
pin_trait_impl!(crate::lptim::OutputPin, LPTIM1, PA14, 1u8);
pin_trait_impl!(crate::lptim::OutputPin, LPTIM1, PB2, 1u8);
pin_trait_impl!(crate::usart::TxPin, LPUART1, PA2, 12u8);
pin_trait_impl!(crate::usart::RxPin, LPUART1, PA3, 12u8);
pin_trait_impl!(crate::usart::CtsPin, LPUART1, PA6, 12u8);
pin_trait_impl!(crate::usart::DePin, LPUART1, PB1, 12u8);
pin_trait_impl!(crate::usart::RtsPin, LPUART1, PB1, 12u8);
pin_trait_impl!(crate::usart::RxPin, LPUART1, PB10, 8u8);
pin_trait_impl!(crate::usart::TxPin, LPUART1, PB11, 8u8);
pin_trait_impl!(crate::usart::DePin, LPUART1, PB12, 8u8);
pin_trait_impl!(crate::usart::RtsPin, LPUART1, PB12, 8u8);
pin_trait_impl!(crate::usart::CtsPin, LPUART1, PB13, 8u8);
impl_opamp_vp_pin!(OPAMP1, PA1, 0u8);
impl_opamp_vout_pin!(OPAMP1, PA2);
impl_opamp_vn_pin!(OPAMP1, PA3, 0u8);
impl_opamp_vp_pin!(OPAMP1, PA3, 1u8);
impl_opamp_vp_pin!(OPAMP1, PA7, 2u8);
impl_opamp_vn_pin!(OPAMP2, PA5, 0u8);
impl_opamp_vout_pin!(OPAMP2, PA6);
impl_opamp_vp_pin!(OPAMP2, PA7, 0u8);
impl_opamp_vp_pin!(OPAMP2, PB0, 2u8);
impl_opamp_vp_pin!(OPAMP2, PB14, 1u8);
impl_opamp_vp_pin!(OPAMP3, PA1, 2u8);
impl_opamp_vp_pin!(OPAMP3, PB0, 0u8);
impl_opamp_vout_pin!(OPAMP3, PB1);
impl_opamp_vn_pin!(OPAMP3, PB10, 1u8);
impl_opamp_vp_pin!(OPAMP3, PB13, 1u8);
impl_opamp_vn_pin!(OPAMP3, PB2, 0u8);
pin_trait_impl!(crate::rcc::McoPin, MCO, PA8, 0u8);
pin_trait_impl!(crate::rcc::McoPin, MCO, PG10, 0u8);
pin_trait_impl!(crate::sai::SdPin<A>, SAI1, PA10, 14u8);
pin_trait_impl!(crate::sai::SdPin<B>, SAI1, PA13, 13u8);
pin_trait_impl!(crate::sai::FsPin<B>, SAI1, PA14, 13u8);
pin_trait_impl!(crate::sai::MclkPin<A>, SAI1, PA3, 13u8);
pin_trait_impl!(crate::sai::FsPin<B>, SAI1, PA4, 13u8);
pin_trait_impl!(crate::sai::SckPin<A>, SAI1, PA8, 14u8);
pin_trait_impl!(crate::sai::FsPin<A>, SAI1, PA9, 14u8);
pin_trait_impl!(crate::sai::SckPin<A>, SAI1, PB10, 14u8);
pin_trait_impl!(crate::sai::SckPin<B>, SAI1, PB3, 14u8);
pin_trait_impl!(crate::sai::MclkPin<B>, SAI1, PB4, 14u8);
pin_trait_impl!(crate::sai::SdPin<B>, SAI1, PB5, 12u8);
pin_trait_impl!(crate::sai::FsPin<B>, SAI1, PB6, 14u8);
pin_trait_impl!(crate::sai::MclkPin<A>, SAI1, PB8, 14u8);
pin_trait_impl!(crate::sai::FsPin<A>, SAI1, PB9, 14u8);
pin_trait_impl!(crate::spi::CsPin, SPI1, PA15, 5u8);
pin_trait_impl!(crate::spi::CsPin, SPI1, PA4, 5u8);
pin_trait_impl!(crate::spi::SckPin, SPI1, PA5, 5u8);
pin_trait_impl!(crate::spi::MisoPin, SPI1, PA6, 5u8);
pin_trait_impl!(crate::spi::MosiPin, SPI1, PA7, 5u8);
pin_trait_impl!(crate::spi::SckPin, SPI1, PB3, 5u8);
pin_trait_impl!(crate::spi::MisoPin, SPI1, PB4, 5u8);
pin_trait_impl!(crate::spi::MosiPin, SPI1, PB5, 5u8);
pin_trait_impl!(crate::spi::MisoPin, SPI2, PA10, 5u8);
pin_trait_impl!(crate::spi::MosiPin, SPI2, PA11, 5u8);
pin_trait_impl!(crate::spi::MckPin, SPI2, PA8, 5u8);
pin_trait_impl!(crate::spi::WsPin, SPI2, PB12, 5u8);
pin_trait_impl!(crate::spi::CsPin, SPI2, PB12, 5u8);
pin_trait_impl!(crate::spi::CkPin, SPI2, PB13, 5u8);
pin_trait_impl!(crate::spi::SckPin, SPI2, PB13, 5u8);
pin_trait_impl!(crate::spi::MisoPin, SPI2, PB14, 5u8);
pin_trait_impl!(crate::spi::MosiPin, SPI2, PB15, 5u8);
pin_trait_impl!(crate::spi::MckPin, SPI2, PC6, 6u8);
pin_trait_impl!(crate::spi::WsPin, SPI2, PF0, 5u8);
pin_trait_impl!(crate::spi::CsPin, SPI2, PF0, 5u8);
pin_trait_impl!(crate::spi::CkPin, SPI2, PF1, 5u8);
pin_trait_impl!(crate::spi::SckPin, SPI2, PF1, 5u8);
pin_trait_impl!(crate::spi::WsPin, SPI3, PA15, 6u8);
pin_trait_impl!(crate::spi::CsPin, SPI3, PA15, 6u8);
pin_trait_impl!(crate::spi::WsPin, SPI3, PA4, 6u8);
pin_trait_impl!(crate::spi::CsPin, SPI3, PA4, 6u8);
pin_trait_impl!(crate::spi::MckPin, SPI3, PA9, 5u8);
pin_trait_impl!(crate::spi::CkPin, SPI3, PB3, 6u8);
pin_trait_impl!(crate::spi::SckPin, SPI3, PB3, 6u8);
pin_trait_impl!(crate::spi::MisoPin, SPI3, PB4, 6u8);
pin_trait_impl!(crate::spi::MosiPin, SPI3, PB5, 6u8);
pin_trait_impl!(crate::spi::CkPin, SPI3, PC10, 6u8);
pin_trait_impl!(crate::spi::SckPin, SPI3, PC10, 6u8);
pin_trait_impl!(crate::spi::MisoPin, SPI3, PC11, 6u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM1, PA10, 6u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn2>, TIM1, PA11, 12u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM1, PA11, 6u8);
pin_trait_impl!(crate::timer::TimerPin<Ch4>, TIM1, PA11, 11u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch2>, TIM1, PA12, 6u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM1, PA12, 11u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM1, PA14, 6u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM1, PA15, 9u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM1, PA6, 6u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM1, PA7, 6u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM1, PA8, 6u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM1, PA9, 6u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch2>, TIM1, PB0, 6u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch3>, TIM1, PB1, 6u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM1, PB10, 12u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM1, PB12, 6u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM1, PB13, 6u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch2>, TIM1, PB14, 6u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch3>, TIM1, PB15, 4u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM1, PB8, 12u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch3>, TIM1, PB9, 12u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM1, PC13, 2u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM1, PC13, 4u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM1, PC4, 2u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch3>, TIM1, PF0, 6u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM15, PA1, 9u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM15, PA2, 9u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM15, PA3, 9u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM15, PA9, 9u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM15, PB14, 1u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM15, PB15, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM15, PB15, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM16, PA12, 1u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM16, PA13, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM16, PA6, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM16, PB4, 1u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM16, PB5, 1u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM16, PB6, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM16, PB8, 1u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM17, PA10, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM17, PA7, 1u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM17, PB4, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM17, PB5, 10u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM17, PB7, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM17, PB9, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM2, PA0, 1u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM2, PA0, 14u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM2, PA1, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch4>, TIM2, PA10, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM2, PA15, 1u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM2, PA15, 14u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM2, PA2, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch4>, TIM2, PA3, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM2, PA5, 1u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM2, PA5, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM2, PA9, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM2, PB10, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch4>, TIM2, PB11, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM2, PB3, 1u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM3, PA4, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM3, PA6, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM3, PA7, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM3, PB0, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch4>, TIM3, PB1, 2u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM3, PB3, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM3, PB4, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM3, PB5, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch4>, TIM3, PB7, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM3, PC6, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM4, PA11, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM4, PA12, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM4, PA13, 10u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM4, PA8, 10u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM4, PB3, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM4, PB6, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM4, PB7, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM4, PB8, 2u8);
pin_trait_impl!(crate::timer::TimerPin<Ch4>, TIM4, PB9, 2u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM8, PA0, 9u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM8, PA0, 10u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM8, PA10, 11u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM8, PA14, 5u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM8, PA15, 2u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM8, PA6, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM8, PA7, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch2>, TIM8, PB0, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch3>, TIM8, PB1, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM8, PB3, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch2>, TIM8, PB4, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch3>, TIM8, PB5, 3u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn2>, TIM8, PB6, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM8, PB6, 5u8);
pin_trait_impl!(crate::timer::ExternalTriggerPin, TIM8, PB6, 6u8);
pin_trait_impl!(crate::timer::BreakInputPin<BkIn1>, TIM8, PB7, 5u8);
pin_trait_impl!(crate::timer::TimerPin<Ch2>, TIM8, PB8, 10u8);
pin_trait_impl!(crate::timer::TimerPin<Ch3>, TIM8, PB9, 10u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch1>, TIM8, PC10, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch2>, TIM8, PC11, 4u8);
pin_trait_impl!(crate::timer::TimerComplementaryPin<Ch4>, TIM8, PC13, 6u8);
pin_trait_impl!(crate::timer::TimerPin<Ch1>, TIM8, PC6, 4u8);
pin_trait_impl!(crate::usart::DePin, UART4, PA15, 8u8);
pin_trait_impl!(crate::usart::RtsPin, UART4, PA15, 8u8);
pin_trait_impl!(crate::usart::CtsPin, UART4, PB7, 14u8);
pin_trait_impl!(crate::usart::TxPin, UART4, PC10, 5u8);
pin_trait_impl!(crate::usart::RxPin, UART4, PC11, 5u8);
pin_trait_impl!(crate::ucpd::Cc2Pin, UCPD1, PB4, 0u8);
pin_trait_impl!(crate::ucpd::Cc1Pin, UCPD1, PB6, 0u8);
pin_trait_impl!(crate::usart::RxPin, USART1, PA10, 7u8);
pin_trait_impl!(crate::usart::CtsPin, USART1, PA11, 7u8);
pin_trait_impl!(crate::usart::DePin, USART1, PA12, 7u8);
pin_trait_impl!(crate::usart::RtsPin, USART1, PA12, 7u8);
pin_trait_impl!(crate::usart::CkPin, USART1, PA8, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART1, PA9, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART1, PB6, 7u8);
pin_trait_impl!(crate::usart::RxPin, USART1, PB7, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART1, PC4, 7u8);
pin_trait_impl!(crate::usart::CtsPin, USART2, PA0, 7u8);
pin_trait_impl!(crate::usart::DePin, USART2, PA1, 7u8);
pin_trait_impl!(crate::usart::RtsPin, USART2, PA1, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART2, PA14, 7u8);
pin_trait_impl!(crate::usart::RxPin, USART2, PA15, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART2, PA2, 7u8);
pin_trait_impl!(crate::usart::RxPin, USART2, PA3, 7u8);
pin_trait_impl!(crate::usart::CkPin, USART2, PA4, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART2, PB3, 7u8);
pin_trait_impl!(crate::usart::RxPin, USART2, PB4, 7u8);
pin_trait_impl!(crate::usart::CkPin, USART2, PB5, 7u8);
pin_trait_impl!(crate::usart::CtsPin, USART3, PA13, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART3, PB10, 7u8);
pin_trait_impl!(crate::usart::RxPin, USART3, PB11, 7u8);
pin_trait_impl!(crate::usart::CkPin, USART3, PB12, 7u8);
pin_trait_impl!(crate::usart::CtsPin, USART3, PB13, 7u8);
pin_trait_impl!(crate::usart::DePin, USART3, PB14, 7u8);
pin_trait_impl!(crate::usart::RtsPin, USART3, PB14, 7u8);
pin_trait_impl!(crate::usart::RxPin, USART3, PB8, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART3, PB9, 7u8);
pin_trait_impl!(crate::usart::TxPin, USART3, PC10, 7u8);
pin_trait_impl!(crate::usart::RxPin, USART3, PC11, 7u8);
pin_trait_impl!(crate::usb::DmPin, USB, PA11, 0u8);
pin_trait_impl!(crate::usb::DpPin, USB, PA12, 0u8);
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA1_CH1, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA1_CH2, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA1_CH3, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA1_CH4, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA1_CH5, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA1_CH6, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA2_CH1, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA2_CH2, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA2_CH3, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA2_CH4, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA2_CH5, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC1, DMA2_CH6, 5u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA1_CH1, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA1_CH2, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA1_CH3, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA1_CH4, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA1_CH5, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA1_CH6, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA2_CH1, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA2_CH2, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA2_CH3, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA2_CH4, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA2_CH5, 36u8, {});
dma_trait_impl!(crate::adc::RxDma, ADC2, DMA2_CH6, 36u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA1_CH1, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA1_CH2, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA1_CH3, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA1_CH4, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA1_CH5, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA1_CH6, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA2_CH1, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA2_CH2, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA2_CH3, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA2_CH4, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA2_CH5, 112u8, {});
dma_trait_impl!(crate::cordic::ReadDma, CORDIC, DMA2_CH6, 112u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA1_CH1, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA1_CH2, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA1_CH3, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA1_CH4, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA1_CH5, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA1_CH6, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA2_CH1, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA2_CH2, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA2_CH3, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA2_CH4, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA2_CH5, 113u8, {});
dma_trait_impl!(crate::cordic::WriteDma, CORDIC, DMA2_CH6, 113u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA1_CH1, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA1_CH2, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA1_CH3, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA1_CH4, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA1_CH5, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA1_CH6, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA2_CH1, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA2_CH2, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA2_CH3, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA2_CH4, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA2_CH5, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC1, DMA2_CH6, 6u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA1_CH1, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA1_CH2, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA1_CH3, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA1_CH4, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA1_CH5, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA1_CH6, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA2_CH1, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA2_CH2, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA2_CH3, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA2_CH4, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA2_CH5, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC1, DMA2_CH6, 7u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA1_CH1, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA1_CH2, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA1_CH3, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA1_CH4, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA1_CH5, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA1_CH6, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA2_CH1, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA2_CH2, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA2_CH3, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA2_CH4, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA2_CH5, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch1>, DAC3, DMA2_CH6, 102u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA1_CH1, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA1_CH2, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA1_CH3, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA1_CH4, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA1_CH5, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA1_CH6, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA2_CH1, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA2_CH2, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA2_CH3, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA2_CH4, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA2_CH5, 103u8, {});
dma_trait_impl!(crate::dac::Dma<Ch2>, DAC3, DMA2_CH6, 103u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA1_CH1, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA1_CH2, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA1_CH3, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA1_CH4, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA1_CH5, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA1_CH6, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA2_CH1, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA2_CH2, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA2_CH3, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA2_CH4, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA2_CH5, 16u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C1, DMA2_CH6, 16u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA1_CH1, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA1_CH2, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA1_CH3, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA1_CH4, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA1_CH5, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA1_CH6, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA2_CH1, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA2_CH2, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA2_CH3, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA2_CH4, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA2_CH5, 17u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C1, DMA2_CH6, 17u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA1_CH1, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA1_CH2, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA1_CH3, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA1_CH4, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA1_CH5, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA1_CH6, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA2_CH1, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA2_CH2, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA2_CH3, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA2_CH4, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA2_CH5, 18u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C2, DMA2_CH6, 18u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA1_CH1, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA1_CH2, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA1_CH3, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA1_CH4, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA1_CH5, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA1_CH6, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA2_CH1, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA2_CH2, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA2_CH3, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA2_CH4, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA2_CH5, 19u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C2, DMA2_CH6, 19u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA1_CH1, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA1_CH2, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA1_CH3, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA1_CH4, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA1_CH5, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA1_CH6, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA2_CH1, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA2_CH2, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA2_CH3, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA2_CH4, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA2_CH5, 20u8, {});
dma_trait_impl!(crate::i2c::RxDma, I2C3, DMA2_CH6, 20u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA1_CH1, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA1_CH2, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA1_CH3, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA1_CH4, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA1_CH5, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA1_CH6, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA2_CH1, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA2_CH2, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA2_CH3, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA2_CH4, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA2_CH5, 21u8, {});
dma_trait_impl!(crate::i2c::TxDma, I2C3, DMA2_CH6, 21u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA1_CH1, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA1_CH2, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA1_CH3, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA1_CH4, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA1_CH5, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA1_CH6, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA2_CH1, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA2_CH2, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA2_CH3, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA2_CH4, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA2_CH5, 34u8, {});
dma_trait_impl!(crate::usart::RxDma, LPUART1, DMA2_CH6, 34u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA1_CH1, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA1_CH2, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA1_CH3, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA1_CH4, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA1_CH5, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA1_CH6, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA2_CH1, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA2_CH2, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA2_CH3, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA2_CH4, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA2_CH5, 35u8, {});
dma_trait_impl!(crate::usart::TxDma, LPUART1, DMA2_CH6, 35u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA1_CH1, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA1_CH2, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA1_CH3, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA1_CH4, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA1_CH5, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA1_CH6, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA2_CH1, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA2_CH2, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA2_CH3, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA2_CH4, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA2_CH5, 108u8, {});
dma_trait_impl!(crate::sai::Dma<A>, SAI1, DMA2_CH6, 108u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA1_CH1, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA1_CH2, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA1_CH3, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA1_CH4, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA1_CH5, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA1_CH6, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA2_CH1, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA2_CH2, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA2_CH3, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA2_CH4, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA2_CH5, 109u8, {});
dma_trait_impl!(crate::sai::Dma<B>, SAI1, DMA2_CH6, 109u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA1_CH1, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA1_CH2, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA1_CH3, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA1_CH4, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA1_CH5, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA1_CH6, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA2_CH1, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA2_CH2, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA2_CH3, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA2_CH4, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA2_CH5, 10u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI1, DMA2_CH6, 10u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA1_CH1, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA1_CH2, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA1_CH3, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA1_CH4, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA1_CH5, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA1_CH6, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA2_CH1, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA2_CH2, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA2_CH3, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA2_CH4, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA2_CH5, 11u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI1, DMA2_CH6, 11u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA1_CH1, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA1_CH2, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA1_CH3, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA1_CH4, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA1_CH5, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA1_CH6, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA2_CH1, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA2_CH2, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA2_CH3, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA2_CH4, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA2_CH5, 12u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI2, DMA2_CH6, 12u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA1_CH1, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA1_CH2, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA1_CH3, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA1_CH4, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA1_CH5, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA1_CH6, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA2_CH1, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA2_CH2, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA2_CH3, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA2_CH4, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA2_CH5, 13u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI2, DMA2_CH6, 13u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA1_CH1, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA1_CH2, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA1_CH3, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA1_CH4, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA1_CH5, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA1_CH6, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA2_CH1, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA2_CH2, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA2_CH3, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA2_CH4, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA2_CH5, 14u8, {});
dma_trait_impl!(crate::spi::RxDma, SPI3, DMA2_CH6, 14u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA1_CH1, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA1_CH2, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA1_CH3, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA1_CH4, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA1_CH5, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA1_CH6, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA2_CH1, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA2_CH2, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA2_CH3, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA2_CH4, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA2_CH5, 15u8, {});
dma_trait_impl!(crate::spi::TxDma, SPI3, DMA2_CH6, 15u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA1_CH1, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA1_CH2, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA1_CH3, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA1_CH4, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA1_CH5, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA1_CH6, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA2_CH1, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA2_CH2, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA2_CH3, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA2_CH4, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA2_CH5, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM1, DMA2_CH6, 42u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA1_CH1, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA1_CH2, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA1_CH3, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA1_CH4, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA1_CH5, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA1_CH6, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA2_CH1, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA2_CH2, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA2_CH3, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA2_CH4, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA2_CH5, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM1, DMA2_CH6, 43u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA1_CH1, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA1_CH2, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA1_CH3, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA1_CH4, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA1_CH5, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA1_CH6, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA2_CH1, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA2_CH2, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA2_CH3, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA2_CH4, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA2_CH5, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM1, DMA2_CH6, 44u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA1_CH1, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA1_CH2, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA1_CH3, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA1_CH4, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA1_CH5, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA1_CH6, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA2_CH1, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA2_CH2, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA2_CH3, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA2_CH4, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA2_CH5, 45u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM1, DMA2_CH6, 45u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA1_CH1, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA1_CH2, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA1_CH3, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA1_CH4, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA1_CH5, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA1_CH6, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA2_CH1, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA2_CH2, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA2_CH3, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA2_CH4, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA2_CH5, 46u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM1, DMA2_CH6, 46u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA1_CH1, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA1_CH2, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA1_CH3, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA1_CH4, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA1_CH5, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA1_CH6, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA2_CH1, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA2_CH2, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA2_CH3, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA2_CH4, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA2_CH5, 78u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM15, DMA2_CH6, 78u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA1_CH1, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA1_CH2, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA1_CH3, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA1_CH4, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA1_CH5, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA1_CH6, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA2_CH1, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA2_CH2, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA2_CH3, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA2_CH4, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA2_CH5, 79u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM15, DMA2_CH6, 79u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA1_CH1, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA1_CH2, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA1_CH3, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA1_CH4, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA1_CH5, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA1_CH6, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA2_CH1, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA2_CH2, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA2_CH3, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA2_CH4, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA2_CH5, 82u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM16, DMA2_CH6, 82u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA1_CH1, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA1_CH2, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA1_CH3, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA1_CH4, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA1_CH5, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA1_CH6, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA2_CH1, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA2_CH2, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA2_CH3, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA2_CH4, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA2_CH5, 83u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM16, DMA2_CH6, 83u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA1_CH1, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA1_CH2, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA1_CH3, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA1_CH4, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA1_CH5, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA1_CH6, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA2_CH1, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA2_CH2, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA2_CH3, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA2_CH4, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA2_CH5, 84u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM17, DMA2_CH6, 84u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA1_CH1, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA1_CH2, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA1_CH3, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA1_CH4, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA1_CH5, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA1_CH6, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA2_CH1, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA2_CH2, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA2_CH3, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA2_CH4, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA2_CH5, 85u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM17, DMA2_CH6, 85u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA1_CH1, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA1_CH2, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA1_CH3, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA1_CH4, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA1_CH5, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA1_CH6, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA2_CH1, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA2_CH2, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA2_CH3, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA2_CH4, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA2_CH5, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM2, DMA2_CH6, 56u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA1_CH1, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA1_CH2, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA1_CH3, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA1_CH4, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA1_CH5, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA1_CH6, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA2_CH1, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA2_CH2, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA2_CH3, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA2_CH4, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA2_CH5, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM2, DMA2_CH6, 57u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA1_CH1, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA1_CH2, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA1_CH3, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA1_CH4, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA1_CH5, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA1_CH6, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA2_CH1, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA2_CH2, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA2_CH3, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA2_CH4, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA2_CH5, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM2, DMA2_CH6, 58u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA1_CH1, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA1_CH2, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA1_CH3, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA1_CH4, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA1_CH5, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA1_CH6, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA2_CH1, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA2_CH2, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA2_CH3, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA2_CH4, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA2_CH5, 59u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM2, DMA2_CH6, 59u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA1_CH1, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA1_CH2, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA1_CH3, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA1_CH4, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA1_CH5, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA1_CH6, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA2_CH1, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA2_CH2, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA2_CH3, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA2_CH4, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA2_CH5, 60u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM2, DMA2_CH6, 60u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA1_CH1, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA1_CH2, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA1_CH3, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA1_CH4, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA1_CH5, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA1_CH6, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA2_CH1, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA2_CH2, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA2_CH3, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA2_CH4, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA2_CH5, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM3, DMA2_CH6, 61u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA1_CH1, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA1_CH2, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA1_CH3, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA1_CH4, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA1_CH5, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA1_CH6, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA2_CH1, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA2_CH2, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA2_CH3, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA2_CH4, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA2_CH5, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM3, DMA2_CH6, 62u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA1_CH1, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA1_CH2, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA1_CH3, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA1_CH4, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA1_CH5, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA1_CH6, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA2_CH1, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA2_CH2, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA2_CH3, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA2_CH4, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA2_CH5, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM3, DMA2_CH6, 63u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA1_CH1, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA1_CH2, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA1_CH3, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA1_CH4, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA1_CH5, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA1_CH6, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA2_CH1, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA2_CH2, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA2_CH3, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA2_CH4, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA2_CH5, 64u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM3, DMA2_CH6, 64u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA1_CH1, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA1_CH2, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA1_CH3, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA1_CH4, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA1_CH5, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA1_CH6, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA2_CH1, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA2_CH2, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA2_CH3, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA2_CH4, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA2_CH5, 65u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM3, DMA2_CH6, 65u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA1_CH1, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA1_CH2, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA1_CH3, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA1_CH4, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA1_CH5, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA1_CH6, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA2_CH1, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA2_CH2, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA2_CH3, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA2_CH4, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA2_CH5, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM4, DMA2_CH6, 67u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA1_CH1, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA1_CH2, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA1_CH3, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA1_CH4, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA1_CH5, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA1_CH6, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA2_CH1, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA2_CH2, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA2_CH3, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA2_CH4, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA2_CH5, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM4, DMA2_CH6, 68u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA1_CH1, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA1_CH2, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA1_CH3, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA1_CH4, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA1_CH5, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA1_CH6, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA2_CH1, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA2_CH2, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA2_CH3, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA2_CH4, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA2_CH5, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM4, DMA2_CH6, 69u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA1_CH1, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA1_CH2, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA1_CH3, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA1_CH4, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA1_CH5, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA1_CH6, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA2_CH1, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA2_CH2, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA2_CH3, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA2_CH4, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA2_CH5, 70u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM4, DMA2_CH6, 70u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA1_CH1, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA1_CH2, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA1_CH3, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA1_CH4, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA1_CH5, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA1_CH6, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA2_CH1, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA2_CH2, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA2_CH3, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA2_CH4, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA2_CH5, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM4, DMA2_CH6, 71u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA1_CH1, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA1_CH2, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA1_CH3, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA1_CH4, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA1_CH5, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA1_CH6, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA2_CH1, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA2_CH2, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA2_CH3, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA2_CH4, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA2_CH5, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM6, DMA2_CH6, 8u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA1_CH1, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA1_CH2, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA1_CH3, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA1_CH4, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA1_CH5, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA1_CH6, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA2_CH1, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA2_CH2, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA2_CH3, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA2_CH4, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA2_CH5, 9u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM7, DMA2_CH6, 9u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA1_CH1, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA1_CH2, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA1_CH3, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA1_CH4, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA1_CH5, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA1_CH6, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA2_CH1, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA2_CH2, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA2_CH3, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA2_CH4, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA2_CH5, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch1>, TIM8, DMA2_CH6, 49u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA1_CH1, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA1_CH2, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA1_CH3, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA1_CH4, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA1_CH5, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA1_CH6, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA2_CH1, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA2_CH2, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA2_CH3, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA2_CH4, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA2_CH5, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch2>, TIM8, DMA2_CH6, 50u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA1_CH1, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA1_CH2, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA1_CH3, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA1_CH4, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA1_CH5, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA1_CH6, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA2_CH1, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA2_CH2, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA2_CH3, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA2_CH4, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA2_CH5, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch3>, TIM8, DMA2_CH6, 51u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA1_CH1, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA1_CH2, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA1_CH3, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA1_CH4, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA1_CH5, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA1_CH6, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA2_CH1, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA2_CH2, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA2_CH3, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA2_CH4, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA2_CH5, 52u8, {});
dma_trait_impl!(crate::timer::Dma<Ch4>, TIM8, DMA2_CH6, 52u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA1_CH1, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA1_CH2, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA1_CH3, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA1_CH4, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA1_CH5, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA1_CH6, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA2_CH1, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA2_CH2, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA2_CH3, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA2_CH4, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA2_CH5, 53u8, {});
dma_trait_impl!(crate::timer::UpDma, TIM8, DMA2_CH6, 53u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA1_CH1, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA1_CH2, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA1_CH3, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA1_CH4, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA1_CH5, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA1_CH6, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA2_CH1, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA2_CH2, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA2_CH3, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA2_CH4, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA2_CH5, 30u8, {});
dma_trait_impl!(crate::usart::RxDma, UART4, DMA2_CH6, 30u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA1_CH1, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA1_CH2, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA1_CH3, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA1_CH4, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA1_CH5, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA1_CH6, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA2_CH1, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA2_CH2, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA2_CH3, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA2_CH4, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA2_CH5, 31u8, {});
dma_trait_impl!(crate::usart::TxDma, UART4, DMA2_CH6, 31u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA1_CH1, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA1_CH2, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA1_CH3, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA1_CH4, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA1_CH5, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA1_CH6, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA2_CH1, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA2_CH2, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA2_CH3, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA2_CH4, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA2_CH5, 114u8, {});
dma_trait_impl!(crate::ucpd::RxDma, UCPD1, DMA2_CH6, 114u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA1_CH1, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA1_CH2, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA1_CH3, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA1_CH4, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA1_CH5, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA1_CH6, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA2_CH1, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA2_CH2, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA2_CH3, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA2_CH4, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA2_CH5, 115u8, {});
dma_trait_impl!(crate::ucpd::TxDma, UCPD1, DMA2_CH6, 115u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA1_CH1, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA1_CH2, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA1_CH3, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA1_CH4, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA1_CH5, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA1_CH6, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA2_CH1, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA2_CH2, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA2_CH3, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA2_CH4, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA2_CH5, 24u8, {});
dma_trait_impl!(crate::usart::RxDma, USART1, DMA2_CH6, 24u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA1_CH1, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA1_CH2, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA1_CH3, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA1_CH4, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA1_CH5, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA1_CH6, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA2_CH1, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA2_CH2, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA2_CH3, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA2_CH4, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA2_CH5, 25u8, {});
dma_trait_impl!(crate::usart::TxDma, USART1, DMA2_CH6, 25u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA1_CH1, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA1_CH2, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA1_CH3, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA1_CH4, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA1_CH5, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA1_CH6, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA2_CH1, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA2_CH2, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA2_CH3, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA2_CH4, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA2_CH5, 26u8, {});
dma_trait_impl!(crate::usart::RxDma, USART2, DMA2_CH6, 26u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA1_CH1, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA1_CH2, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA1_CH3, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA1_CH4, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA1_CH5, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA1_CH6, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA2_CH1, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA2_CH2, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA2_CH3, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA2_CH4, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA2_CH5, 27u8, {});
dma_trait_impl!(crate::usart::TxDma, USART2, DMA2_CH6, 27u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA1_CH1, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA1_CH2, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA1_CH3, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA1_CH4, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA1_CH5, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA1_CH6, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA2_CH1, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA2_CH2, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA2_CH3, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA2_CH4, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA2_CH5, 28u8, {});
dma_trait_impl!(crate::usart::RxDma, USART3, DMA2_CH6, 28u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA1_CH1, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA1_CH2, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA1_CH3, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA1_CH4, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA1_CH5, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA1_CH6, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA2_CH1, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA2_CH2, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA2_CH3, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA2_CH4, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA2_CH5, 29u8, {});
dma_trait_impl!(crate::usart::TxDma, USART3, DMA2_CH6, 29u8, {});
impl core::ops::Div<crate::pac::rcc::vals::Hpre> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Hpre) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Hpre::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV2 => self * 1u32 / 2u32,
            crate::pac::rcc::vals::Hpre::DIV4 => self * 1u32 / 4u32,
            crate::pac::rcc::vals::Hpre::DIV8 => self * 1u32 / 8u32,
            crate::pac::rcc::vals::Hpre::DIV16 => self * 1u32 / 16u32,
            crate::pac::rcc::vals::Hpre::DIV64 => self * 1u32 / 64u32,
            crate::pac::rcc::vals::Hpre::DIV128 => self * 1u32 / 128u32,
            crate::pac::rcc::vals::Hpre::DIV256 => self * 1u32 / 256u32,
            crate::pac::rcc::vals::Hpre::DIV512 => self * 1u32 / 512u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Hpre> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Hpre) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Hpre::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV2 => self * 2u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV4 => self * 4u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV8 => self * 8u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV16 => self * 16u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV64 => self * 64u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV128 => self * 128u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV256 => self * 256u32 / 1u32,
            crate::pac::rcc::vals::Hpre::DIV512 => self * 512u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Div<crate::pac::rcc::vals::Mcopre> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Mcopre) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Mcopre::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Mcopre::DIV2 => self * 1u32 / 2u32,
            crate::pac::rcc::vals::Mcopre::DIV4 => self * 1u32 / 4u32,
            crate::pac::rcc::vals::Mcopre::DIV8 => self * 1u32 / 8u32,
            crate::pac::rcc::vals::Mcopre::DIV16 => self * 1u32 / 16u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Mcopre> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Mcopre) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Mcopre::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Mcopre::DIV2 => self * 2u32 / 1u32,
            crate::pac::rcc::vals::Mcopre::DIV4 => self * 4u32 / 1u32,
            crate::pac::rcc::vals::Mcopre::DIV8 => self * 8u32 / 1u32,
            crate::pac::rcc::vals::Mcopre::DIV16 => self * 16u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Div<crate::pac::rcc::vals::Pllm> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Pllm) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllm::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV2 => self * 1u32 / 2u32,
            crate::pac::rcc::vals::Pllm::DIV3 => self * 1u32 / 3u32,
            crate::pac::rcc::vals::Pllm::DIV4 => self * 1u32 / 4u32,
            crate::pac::rcc::vals::Pllm::DIV5 => self * 1u32 / 5u32,
            crate::pac::rcc::vals::Pllm::DIV6 => self * 1u32 / 6u32,
            crate::pac::rcc::vals::Pllm::DIV7 => self * 1u32 / 7u32,
            crate::pac::rcc::vals::Pllm::DIV8 => self * 1u32 / 8u32,
            crate::pac::rcc::vals::Pllm::DIV9 => self * 1u32 / 9u32,
            crate::pac::rcc::vals::Pllm::DIV10 => self * 1u32 / 10u32,
            crate::pac::rcc::vals::Pllm::DIV11 => self * 1u32 / 11u32,
            crate::pac::rcc::vals::Pllm::DIV12 => self * 1u32 / 12u32,
            crate::pac::rcc::vals::Pllm::DIV13 => self * 1u32 / 13u32,
            crate::pac::rcc::vals::Pllm::DIV14 => self * 1u32 / 14u32,
            crate::pac::rcc::vals::Pllm::DIV15 => self * 1u32 / 15u32,
            crate::pac::rcc::vals::Pllm::DIV16 => self * 1u32 / 16u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Pllm> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Pllm) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllm::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV2 => self * 2u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV3 => self * 3u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV4 => self * 4u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV5 => self * 5u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV6 => self * 6u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV7 => self * 7u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV8 => self * 8u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV9 => self * 9u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV10 => self * 10u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV11 => self * 11u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV12 => self * 12u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV13 => self * 13u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV14 => self * 14u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV15 => self * 15u32 / 1u32,
            crate::pac::rcc::vals::Pllm::DIV16 => self * 16u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Div<crate::pac::rcc::vals::Plln> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Plln) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Plln::MUL8 => self * 1u32 / 8u32,
            crate::pac::rcc::vals::Plln::MUL9 => self * 1u32 / 9u32,
            crate::pac::rcc::vals::Plln::MUL10 => self * 1u32 / 10u32,
            crate::pac::rcc::vals::Plln::MUL11 => self * 1u32 / 11u32,
            crate::pac::rcc::vals::Plln::MUL12 => self * 1u32 / 12u32,
            crate::pac::rcc::vals::Plln::MUL13 => self * 1u32 / 13u32,
            crate::pac::rcc::vals::Plln::MUL14 => self * 1u32 / 14u32,
            crate::pac::rcc::vals::Plln::MUL15 => self * 1u32 / 15u32,
            crate::pac::rcc::vals::Plln::MUL16 => self * 1u32 / 16u32,
            crate::pac::rcc::vals::Plln::MUL17 => self * 1u32 / 17u32,
            crate::pac::rcc::vals::Plln::MUL18 => self * 1u32 / 18u32,
            crate::pac::rcc::vals::Plln::MUL19 => self * 1u32 / 19u32,
            crate::pac::rcc::vals::Plln::MUL20 => self * 1u32 / 20u32,
            crate::pac::rcc::vals::Plln::MUL21 => self * 1u32 / 21u32,
            crate::pac::rcc::vals::Plln::MUL22 => self * 1u32 / 22u32,
            crate::pac::rcc::vals::Plln::MUL23 => self * 1u32 / 23u32,
            crate::pac::rcc::vals::Plln::MUL24 => self * 1u32 / 24u32,
            crate::pac::rcc::vals::Plln::MUL25 => self * 1u32 / 25u32,
            crate::pac::rcc::vals::Plln::MUL26 => self * 1u32 / 26u32,
            crate::pac::rcc::vals::Plln::MUL27 => self * 1u32 / 27u32,
            crate::pac::rcc::vals::Plln::MUL28 => self * 1u32 / 28u32,
            crate::pac::rcc::vals::Plln::MUL29 => self * 1u32 / 29u32,
            crate::pac::rcc::vals::Plln::MUL30 => self * 1u32 / 30u32,
            crate::pac::rcc::vals::Plln::MUL31 => self * 1u32 / 31u32,
            crate::pac::rcc::vals::Plln::MUL32 => self * 1u32 / 32u32,
            crate::pac::rcc::vals::Plln::MUL33 => self * 1u32 / 33u32,
            crate::pac::rcc::vals::Plln::MUL34 => self * 1u32 / 34u32,
            crate::pac::rcc::vals::Plln::MUL35 => self * 1u32 / 35u32,
            crate::pac::rcc::vals::Plln::MUL36 => self * 1u32 / 36u32,
            crate::pac::rcc::vals::Plln::MUL37 => self * 1u32 / 37u32,
            crate::pac::rcc::vals::Plln::MUL38 => self * 1u32 / 38u32,
            crate::pac::rcc::vals::Plln::MUL39 => self * 1u32 / 39u32,
            crate::pac::rcc::vals::Plln::MUL40 => self * 1u32 / 40u32,
            crate::pac::rcc::vals::Plln::MUL41 => self * 1u32 / 41u32,
            crate::pac::rcc::vals::Plln::MUL42 => self * 1u32 / 42u32,
            crate::pac::rcc::vals::Plln::MUL43 => self * 1u32 / 43u32,
            crate::pac::rcc::vals::Plln::MUL44 => self * 1u32 / 44u32,
            crate::pac::rcc::vals::Plln::MUL45 => self * 1u32 / 45u32,
            crate::pac::rcc::vals::Plln::MUL46 => self * 1u32 / 46u32,
            crate::pac::rcc::vals::Plln::MUL47 => self * 1u32 / 47u32,
            crate::pac::rcc::vals::Plln::MUL48 => self * 1u32 / 48u32,
            crate::pac::rcc::vals::Plln::MUL49 => self * 1u32 / 49u32,
            crate::pac::rcc::vals::Plln::MUL50 => self * 1u32 / 50u32,
            crate::pac::rcc::vals::Plln::MUL51 => self * 1u32 / 51u32,
            crate::pac::rcc::vals::Plln::MUL52 => self * 1u32 / 52u32,
            crate::pac::rcc::vals::Plln::MUL53 => self * 1u32 / 53u32,
            crate::pac::rcc::vals::Plln::MUL54 => self * 1u32 / 54u32,
            crate::pac::rcc::vals::Plln::MUL55 => self * 1u32 / 55u32,
            crate::pac::rcc::vals::Plln::MUL56 => self * 1u32 / 56u32,
            crate::pac::rcc::vals::Plln::MUL57 => self * 1u32 / 57u32,
            crate::pac::rcc::vals::Plln::MUL58 => self * 1u32 / 58u32,
            crate::pac::rcc::vals::Plln::MUL59 => self * 1u32 / 59u32,
            crate::pac::rcc::vals::Plln::MUL60 => self * 1u32 / 60u32,
            crate::pac::rcc::vals::Plln::MUL61 => self * 1u32 / 61u32,
            crate::pac::rcc::vals::Plln::MUL62 => self * 1u32 / 62u32,
            crate::pac::rcc::vals::Plln::MUL63 => self * 1u32 / 63u32,
            crate::pac::rcc::vals::Plln::MUL64 => self * 1u32 / 64u32,
            crate::pac::rcc::vals::Plln::MUL65 => self * 1u32 / 65u32,
            crate::pac::rcc::vals::Plln::MUL66 => self * 1u32 / 66u32,
            crate::pac::rcc::vals::Plln::MUL67 => self * 1u32 / 67u32,
            crate::pac::rcc::vals::Plln::MUL68 => self * 1u32 / 68u32,
            crate::pac::rcc::vals::Plln::MUL69 => self * 1u32 / 69u32,
            crate::pac::rcc::vals::Plln::MUL70 => self * 1u32 / 70u32,
            crate::pac::rcc::vals::Plln::MUL71 => self * 1u32 / 71u32,
            crate::pac::rcc::vals::Plln::MUL72 => self * 1u32 / 72u32,
            crate::pac::rcc::vals::Plln::MUL73 => self * 1u32 / 73u32,
            crate::pac::rcc::vals::Plln::MUL74 => self * 1u32 / 74u32,
            crate::pac::rcc::vals::Plln::MUL75 => self * 1u32 / 75u32,
            crate::pac::rcc::vals::Plln::MUL76 => self * 1u32 / 76u32,
            crate::pac::rcc::vals::Plln::MUL77 => self * 1u32 / 77u32,
            crate::pac::rcc::vals::Plln::MUL78 => self * 1u32 / 78u32,
            crate::pac::rcc::vals::Plln::MUL79 => self * 1u32 / 79u32,
            crate::pac::rcc::vals::Plln::MUL80 => self * 1u32 / 80u32,
            crate::pac::rcc::vals::Plln::MUL81 => self * 1u32 / 81u32,
            crate::pac::rcc::vals::Plln::MUL82 => self * 1u32 / 82u32,
            crate::pac::rcc::vals::Plln::MUL83 => self * 1u32 / 83u32,
            crate::pac::rcc::vals::Plln::MUL84 => self * 1u32 / 84u32,
            crate::pac::rcc::vals::Plln::MUL85 => self * 1u32 / 85u32,
            crate::pac::rcc::vals::Plln::MUL86 => self * 1u32 / 86u32,
            crate::pac::rcc::vals::Plln::MUL87 => self * 1u32 / 87u32,
            crate::pac::rcc::vals::Plln::MUL88 => self * 1u32 / 88u32,
            crate::pac::rcc::vals::Plln::MUL89 => self * 1u32 / 89u32,
            crate::pac::rcc::vals::Plln::MUL90 => self * 1u32 / 90u32,
            crate::pac::rcc::vals::Plln::MUL91 => self * 1u32 / 91u32,
            crate::pac::rcc::vals::Plln::MUL92 => self * 1u32 / 92u32,
            crate::pac::rcc::vals::Plln::MUL93 => self * 1u32 / 93u32,
            crate::pac::rcc::vals::Plln::MUL94 => self * 1u32 / 94u32,
            crate::pac::rcc::vals::Plln::MUL95 => self * 1u32 / 95u32,
            crate::pac::rcc::vals::Plln::MUL96 => self * 1u32 / 96u32,
            crate::pac::rcc::vals::Plln::MUL97 => self * 1u32 / 97u32,
            crate::pac::rcc::vals::Plln::MUL98 => self * 1u32 / 98u32,
            crate::pac::rcc::vals::Plln::MUL99 => self * 1u32 / 99u32,
            crate::pac::rcc::vals::Plln::MUL100 => self * 1u32 / 100u32,
            crate::pac::rcc::vals::Plln::MUL101 => self * 1u32 / 101u32,
            crate::pac::rcc::vals::Plln::MUL102 => self * 1u32 / 102u32,
            crate::pac::rcc::vals::Plln::MUL103 => self * 1u32 / 103u32,
            crate::pac::rcc::vals::Plln::MUL104 => self * 1u32 / 104u32,
            crate::pac::rcc::vals::Plln::MUL105 => self * 1u32 / 105u32,
            crate::pac::rcc::vals::Plln::MUL106 => self * 1u32 / 106u32,
            crate::pac::rcc::vals::Plln::MUL107 => self * 1u32 / 107u32,
            crate::pac::rcc::vals::Plln::MUL108 => self * 1u32 / 108u32,
            crate::pac::rcc::vals::Plln::MUL109 => self * 1u32 / 109u32,
            crate::pac::rcc::vals::Plln::MUL110 => self * 1u32 / 110u32,
            crate::pac::rcc::vals::Plln::MUL111 => self * 1u32 / 111u32,
            crate::pac::rcc::vals::Plln::MUL112 => self * 1u32 / 112u32,
            crate::pac::rcc::vals::Plln::MUL113 => self * 1u32 / 113u32,
            crate::pac::rcc::vals::Plln::MUL114 => self * 1u32 / 114u32,
            crate::pac::rcc::vals::Plln::MUL115 => self * 1u32 / 115u32,
            crate::pac::rcc::vals::Plln::MUL116 => self * 1u32 / 116u32,
            crate::pac::rcc::vals::Plln::MUL117 => self * 1u32 / 117u32,
            crate::pac::rcc::vals::Plln::MUL118 => self * 1u32 / 118u32,
            crate::pac::rcc::vals::Plln::MUL119 => self * 1u32 / 119u32,
            crate::pac::rcc::vals::Plln::MUL120 => self * 1u32 / 120u32,
            crate::pac::rcc::vals::Plln::MUL121 => self * 1u32 / 121u32,
            crate::pac::rcc::vals::Plln::MUL122 => self * 1u32 / 122u32,
            crate::pac::rcc::vals::Plln::MUL123 => self * 1u32 / 123u32,
            crate::pac::rcc::vals::Plln::MUL124 => self * 1u32 / 124u32,
            crate::pac::rcc::vals::Plln::MUL125 => self * 1u32 / 125u32,
            crate::pac::rcc::vals::Plln::MUL126 => self * 1u32 / 126u32,
            crate::pac::rcc::vals::Plln::MUL127 => self * 1u32 / 127u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Plln> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Plln) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Plln::MUL8 => self * 8u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL9 => self * 9u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL10 => self * 10u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL11 => self * 11u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL12 => self * 12u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL13 => self * 13u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL14 => self * 14u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL15 => self * 15u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL16 => self * 16u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL17 => self * 17u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL18 => self * 18u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL19 => self * 19u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL20 => self * 20u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL21 => self * 21u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL22 => self * 22u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL23 => self * 23u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL24 => self * 24u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL25 => self * 25u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL26 => self * 26u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL27 => self * 27u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL28 => self * 28u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL29 => self * 29u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL30 => self * 30u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL31 => self * 31u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL32 => self * 32u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL33 => self * 33u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL34 => self * 34u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL35 => self * 35u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL36 => self * 36u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL37 => self * 37u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL38 => self * 38u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL39 => self * 39u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL40 => self * 40u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL41 => self * 41u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL42 => self * 42u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL43 => self * 43u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL44 => self * 44u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL45 => self * 45u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL46 => self * 46u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL47 => self * 47u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL48 => self * 48u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL49 => self * 49u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL50 => self * 50u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL51 => self * 51u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL52 => self * 52u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL53 => self * 53u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL54 => self * 54u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL55 => self * 55u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL56 => self * 56u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL57 => self * 57u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL58 => self * 58u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL59 => self * 59u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL60 => self * 60u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL61 => self * 61u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL62 => self * 62u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL63 => self * 63u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL64 => self * 64u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL65 => self * 65u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL66 => self * 66u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL67 => self * 67u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL68 => self * 68u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL69 => self * 69u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL70 => self * 70u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL71 => self * 71u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL72 => self * 72u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL73 => self * 73u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL74 => self * 74u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL75 => self * 75u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL76 => self * 76u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL77 => self * 77u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL78 => self * 78u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL79 => self * 79u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL80 => self * 80u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL81 => self * 81u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL82 => self * 82u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL83 => self * 83u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL84 => self * 84u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL85 => self * 85u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL86 => self * 86u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL87 => self * 87u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL88 => self * 88u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL89 => self * 89u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL90 => self * 90u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL91 => self * 91u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL92 => self * 92u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL93 => self * 93u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL94 => self * 94u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL95 => self * 95u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL96 => self * 96u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL97 => self * 97u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL98 => self * 98u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL99 => self * 99u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL100 => self * 100u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL101 => self * 101u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL102 => self * 102u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL103 => self * 103u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL104 => self * 104u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL105 => self * 105u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL106 => self * 106u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL107 => self * 107u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL108 => self * 108u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL109 => self * 109u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL110 => self * 110u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL111 => self * 111u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL112 => self * 112u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL113 => self * 113u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL114 => self * 114u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL115 => self * 115u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL116 => self * 116u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL117 => self * 117u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL118 => self * 118u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL119 => self * 119u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL120 => self * 120u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL121 => self * 121u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL122 => self * 122u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL123 => self * 123u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL124 => self * 124u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL125 => self * 125u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL126 => self * 126u32 / 1u32,
            crate::pac::rcc::vals::Plln::MUL127 => self * 127u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Div<crate::pac::rcc::vals::Pllp> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Pllp) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllp::DIV2 => self * 1u32 / 2u32,
            crate::pac::rcc::vals::Pllp::DIV3 => self * 1u32 / 3u32,
            crate::pac::rcc::vals::Pllp::DIV4 => self * 1u32 / 4u32,
            crate::pac::rcc::vals::Pllp::DIV5 => self * 1u32 / 5u32,
            crate::pac::rcc::vals::Pllp::DIV6 => self * 1u32 / 6u32,
            crate::pac::rcc::vals::Pllp::DIV7 => self * 1u32 / 7u32,
            crate::pac::rcc::vals::Pllp::DIV8 => self * 1u32 / 8u32,
            crate::pac::rcc::vals::Pllp::DIV9 => self * 1u32 / 9u32,
            crate::pac::rcc::vals::Pllp::DIV10 => self * 1u32 / 10u32,
            crate::pac::rcc::vals::Pllp::DIV11 => self * 1u32 / 11u32,
            crate::pac::rcc::vals::Pllp::DIV12 => self * 1u32 / 12u32,
            crate::pac::rcc::vals::Pllp::DIV13 => self * 1u32 / 13u32,
            crate::pac::rcc::vals::Pllp::DIV14 => self * 1u32 / 14u32,
            crate::pac::rcc::vals::Pllp::DIV15 => self * 1u32 / 15u32,
            crate::pac::rcc::vals::Pllp::DIV16 => self * 1u32 / 16u32,
            crate::pac::rcc::vals::Pllp::DIV17 => self * 1u32 / 17u32,
            crate::pac::rcc::vals::Pllp::DIV18 => self * 1u32 / 18u32,
            crate::pac::rcc::vals::Pllp::DIV19 => self * 1u32 / 19u32,
            crate::pac::rcc::vals::Pllp::DIV20 => self * 1u32 / 20u32,
            crate::pac::rcc::vals::Pllp::DIV21 => self * 1u32 / 21u32,
            crate::pac::rcc::vals::Pllp::DIV22 => self * 1u32 / 22u32,
            crate::pac::rcc::vals::Pllp::DIV23 => self * 1u32 / 23u32,
            crate::pac::rcc::vals::Pllp::DIV24 => self * 1u32 / 24u32,
            crate::pac::rcc::vals::Pllp::DIV25 => self * 1u32 / 25u32,
            crate::pac::rcc::vals::Pllp::DIV26 => self * 1u32 / 26u32,
            crate::pac::rcc::vals::Pllp::DIV27 => self * 1u32 / 27u32,
            crate::pac::rcc::vals::Pllp::DIV28 => self * 1u32 / 28u32,
            crate::pac::rcc::vals::Pllp::DIV29 => self * 1u32 / 29u32,
            crate::pac::rcc::vals::Pllp::DIV30 => self * 1u32 / 30u32,
            crate::pac::rcc::vals::Pllp::DIV31 => self * 1u32 / 31u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Pllp> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Pllp) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllp::DIV2 => self * 2u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV3 => self * 3u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV4 => self * 4u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV5 => self * 5u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV6 => self * 6u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV7 => self * 7u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV8 => self * 8u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV9 => self * 9u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV10 => self * 10u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV11 => self * 11u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV12 => self * 12u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV13 => self * 13u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV14 => self * 14u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV15 => self * 15u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV16 => self * 16u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV17 => self * 17u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV18 => self * 18u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV19 => self * 19u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV20 => self * 20u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV21 => self * 21u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV22 => self * 22u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV23 => self * 23u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV24 => self * 24u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV25 => self * 25u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV26 => self * 26u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV27 => self * 27u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV28 => self * 28u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV29 => self * 29u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV30 => self * 30u32 / 1u32,
            crate::pac::rcc::vals::Pllp::DIV31 => self * 31u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Div<crate::pac::rcc::vals::Pllq> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Pllq) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllq::DIV2 => self * 1u32 / 2u32,
            crate::pac::rcc::vals::Pllq::DIV4 => self * 1u32 / 4u32,
            crate::pac::rcc::vals::Pllq::DIV6 => self * 1u32 / 6u32,
            crate::pac::rcc::vals::Pllq::DIV8 => self * 1u32 / 8u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Pllq> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Pllq) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllq::DIV2 => self * 2u32 / 1u32,
            crate::pac::rcc::vals::Pllq::DIV4 => self * 4u32 / 1u32,
            crate::pac::rcc::vals::Pllq::DIV6 => self * 6u32 / 1u32,
            crate::pac::rcc::vals::Pllq::DIV8 => self * 8u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Div<crate::pac::rcc::vals::Pllr> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Pllr) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllr::DIV2 => self * 1u32 / 2u32,
            crate::pac::rcc::vals::Pllr::DIV4 => self * 1u32 / 4u32,
            crate::pac::rcc::vals::Pllr::DIV6 => self * 1u32 / 6u32,
            crate::pac::rcc::vals::Pllr::DIV8 => self * 1u32 / 8u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Pllr> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Pllr) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Pllr::DIV2 => self * 2u32 / 1u32,
            crate::pac::rcc::vals::Pllr::DIV4 => self * 4u32 / 1u32,
            crate::pac::rcc::vals::Pllr::DIV6 => self * 6u32 / 1u32,
            crate::pac::rcc::vals::Pllr::DIV8 => self * 8u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Div<crate::pac::rcc::vals::Ppre> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn div(self, rhs: crate::pac::rcc::vals::Ppre) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Ppre::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Ppre::DIV2 => self * 1u32 / 2u32,
            crate::pac::rcc::vals::Ppre::DIV4 => self * 1u32 / 4u32,
            crate::pac::rcc::vals::Ppre::DIV8 => self * 1u32 / 8u32,
            crate::pac::rcc::vals::Ppre::DIV16 => self * 1u32 / 16u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
impl core::ops::Mul<crate::pac::rcc::vals::Ppre> for crate::time::Hertz {
    type Output = crate::time::Hertz;
    fn mul(self, rhs: crate::pac::rcc::vals::Ppre) -> Self::Output {
        match rhs {
            crate::pac::rcc::vals::Ppre::DIV1 => self * 1u32 / 1u32,
            crate::pac::rcc::vals::Ppre::DIV2 => self * 2u32 / 1u32,
            crate::pac::rcc::vals::Ppre::DIV4 => self * 4u32 / 1u32,
            crate::pac::rcc::vals::Ppre::DIV8 => self * 8u32 / 1u32,
            crate::pac::rcc::vals::Ppre::DIV16 => self * 16u32 / 1u32,
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }
}
#[allow(non_camel_case_types)]
pub mod peripheral_interrupts {
    pub mod ADC1 {
        pub type GLOBAL = crate::interrupt::typelevel::ADC1_2;
    }
    pub mod ADC12_COMMON {}
    pub mod ADC2 {
        pub type GLOBAL = crate::interrupt::typelevel::ADC1_2;
    }
    pub mod COMP1 {
        pub type WKUP = crate::interrupt::typelevel::COMP1_2_3;
    }
    pub mod COMP2 {
        pub type WKUP = crate::interrupt::typelevel::COMP1_2_3;
    }
    pub mod COMP3 {
        pub type WKUP = crate::interrupt::typelevel::COMP1_2_3;
    }
    pub mod COMP4 {
        pub type WKUP = crate::interrupt::typelevel::COMP4;
    }
    pub mod CORDIC {
        pub type GLOBAL = crate::interrupt::typelevel::CORDIC;
    }
    pub mod CRC {}
    pub mod CRS {}
    pub mod DAC1 {
        pub type GLOBAL = crate::interrupt::typelevel::TIM6_DAC;
    }
    pub mod DAC3 {
        pub type GLOBAL = crate::interrupt::typelevel::TIM6_DAC;
    }
    pub mod DBGMCU {}
    pub mod DMA1 {
        pub type CH1 = crate::interrupt::typelevel::DMA1_CHANNEL1;
        pub type CH2 = crate::interrupt::typelevel::DMA1_CHANNEL2;
        pub type CH3 = crate::interrupt::typelevel::DMA1_CHANNEL3;
        pub type CH4 = crate::interrupt::typelevel::DMA1_CHANNEL4;
        pub type CH5 = crate::interrupt::typelevel::DMA1_CHANNEL5;
        pub type CH6 = crate::interrupt::typelevel::DMA1_CHANNEL6;
    }
    pub mod DMA2 {
        pub type CH1 = crate::interrupt::typelevel::DMA2_CHANNEL1;
        pub type CH2 = crate::interrupt::typelevel::DMA2_CHANNEL2;
        pub type CH3 = crate::interrupt::typelevel::DMA2_CHANNEL3;
        pub type CH4 = crate::interrupt::typelevel::DMA2_CHANNEL4;
        pub type CH5 = crate::interrupt::typelevel::DMA2_CHANNEL5;
        pub type CH6 = crate::interrupt::typelevel::DMA2_CHANNEL6;
    }
    pub mod DMAMUX1 {
        pub type OVR = crate::interrupt::typelevel::DMAMUX_OVR;
    }
    pub mod EXTI {
        pub type EXTI0 = crate::interrupt::typelevel::EXTI0;
        pub type EXTI1 = crate::interrupt::typelevel::EXTI1;
        pub type EXTI10 = crate::interrupt::typelevel::EXTI15_10;
        pub type EXTI11 = crate::interrupt::typelevel::EXTI15_10;
        pub type EXTI12 = crate::interrupt::typelevel::EXTI15_10;
        pub type EXTI13 = crate::interrupt::typelevel::EXTI15_10;
        pub type EXTI14 = crate::interrupt::typelevel::EXTI15_10;
        pub type EXTI15 = crate::interrupt::typelevel::EXTI15_10;
        pub type EXTI2 = crate::interrupt::typelevel::EXTI2;
        pub type EXTI3 = crate::interrupt::typelevel::EXTI3;
        pub type EXTI4 = crate::interrupt::typelevel::EXTI4;
        pub type EXTI5 = crate::interrupt::typelevel::EXTI9_5;
        pub type EXTI6 = crate::interrupt::typelevel::EXTI9_5;
        pub type EXTI7 = crate::interrupt::typelevel::EXTI9_5;
        pub type EXTI8 = crate::interrupt::typelevel::EXTI9_5;
        pub type EXTI9 = crate::interrupt::typelevel::EXTI9_5;
    }
    pub mod FDCAN1 {
        pub type IT0 = crate::interrupt::typelevel::FDCAN1_IT0;
        pub type IT1 = crate::interrupt::typelevel::FDCAN1_IT1;
    }
    pub mod FDCANRAM1 {}
    pub mod FLASH {
        pub type GLOBAL = crate::interrupt::typelevel::FLASH;
    }
    pub mod FMAC {
        pub type GLOBAL = crate::interrupt::typelevel::FMAC;
    }
    pub mod GPIOA {}
    pub mod GPIOB {}
    pub mod GPIOC {}
    pub mod GPIOD {}
    pub mod GPIOE {}
    pub mod GPIOF {}
    pub mod GPIOG {}
    pub mod I2C1 {
        pub type ER = crate::interrupt::typelevel::I2C1_ER;
        pub type EV = crate::interrupt::typelevel::I2C1_EV;
    }
    pub mod I2C2 {
        pub type ER = crate::interrupt::typelevel::I2C2_ER;
        pub type EV = crate::interrupt::typelevel::I2C2_EV;
    }
    pub mod I2C3 {
        pub type ER = crate::interrupt::typelevel::I2C3_ER;
        pub type EV = crate::interrupt::typelevel::I2C3_EV;
    }
    pub mod IWDG {}
    pub mod LPTIM1 {
        pub type GLOBAL = crate::interrupt::typelevel::LPTIM1;
    }
    pub mod LPUART1 {
        pub type GLOBAL = crate::interrupt::typelevel::LPUART1;
    }
    pub mod OPAMP1 {}
    pub mod OPAMP2 {}
    pub mod OPAMP3 {}
    pub mod PWR {}
    pub mod RCC {
        pub type CRS = crate::interrupt::typelevel::CRS;
        pub type GLOBAL = crate::interrupt::typelevel::RCC;
        pub type LSECSS = crate::interrupt::typelevel::RTC_TAMP_LSECSS;
    }
    pub mod RNG {
        pub type GLOBAL = crate::interrupt::typelevel::RNG;
    }
    pub mod RTC {
        pub type ALARM = crate::interrupt::typelevel::RTC_ALARM;
        pub type TAMP = crate::interrupt::typelevel::RTC_TAMP_LSECSS;
        pub type WKUP = crate::interrupt::typelevel::RTC_WKUP;
    }
    pub mod SAI1 {
        pub type A = crate::interrupt::typelevel::SAI1;
        pub type B = crate::interrupt::typelevel::SAI1;
    }
    pub mod SPI1 {
        pub type GLOBAL = crate::interrupt::typelevel::SPI1;
    }
    pub mod SPI2 {
        pub type GLOBAL = crate::interrupt::typelevel::SPI2;
    }
    pub mod SPI3 {
        pub type GLOBAL = crate::interrupt::typelevel::SPI3;
    }
    pub mod SYSCFG {}
    pub mod TAMP {}
    pub mod TIM1 {
        pub type BRK = crate::interrupt::typelevel::TIM1_BRK_TIM15;
        pub type CC = crate::interrupt::typelevel::TIM1_CC;
        pub type COM = crate::interrupt::typelevel::TIM1_TRG_COM_TIM17;
        pub type TRG = crate::interrupt::typelevel::TIM1_TRG_COM_TIM17;
        pub type UP = crate::interrupt::typelevel::TIM1_UP_TIM16;
    }
    pub mod TIM15 {
        pub type BRK = crate::interrupt::typelevel::TIM1_BRK_TIM15;
        pub type CC = crate::interrupt::typelevel::TIM1_BRK_TIM15;
        pub type COM = crate::interrupt::typelevel::TIM1_BRK_TIM15;
        pub type TRG = crate::interrupt::typelevel::TIM1_BRK_TIM15;
        pub type UP = crate::interrupt::typelevel::TIM1_BRK_TIM15;
    }
    pub mod TIM16 {
        pub type BRK = crate::interrupt::typelevel::TIM1_UP_TIM16;
        pub type CC = crate::interrupt::typelevel::TIM1_UP_TIM16;
        pub type COM = crate::interrupt::typelevel::TIM1_UP_TIM16;
        pub type TRG = crate::interrupt::typelevel::TIM1_UP_TIM16;
        pub type UP = crate::interrupt::typelevel::TIM1_UP_TIM16;
    }
    pub mod TIM17 {
        pub type BRK = crate::interrupt::typelevel::TIM1_TRG_COM_TIM17;
        pub type CC = crate::interrupt::typelevel::TIM1_TRG_COM_TIM17;
        pub type COM = crate::interrupt::typelevel::TIM1_TRG_COM_TIM17;
        pub type TRG = crate::interrupt::typelevel::TIM1_TRG_COM_TIM17;
        pub type UP = crate::interrupt::typelevel::TIM1_TRG_COM_TIM17;
    }
    pub mod TIM2 {
        pub type BRK = crate::interrupt::typelevel::TIM2;
        pub type CC = crate::interrupt::typelevel::TIM2;
        pub type COM = crate::interrupt::typelevel::TIM2;
        pub type TRG = crate::interrupt::typelevel::TIM2;
        pub type UP = crate::interrupt::typelevel::TIM2;
    }
    pub mod TIM3 {
        pub type BRK = crate::interrupt::typelevel::TIM3;
        pub type CC = crate::interrupt::typelevel::TIM3;
        pub type COM = crate::interrupt::typelevel::TIM3;
        pub type TRG = crate::interrupt::typelevel::TIM3;
        pub type UP = crate::interrupt::typelevel::TIM3;
    }
    pub mod TIM4 {
        pub type BRK = crate::interrupt::typelevel::TIM4;
        pub type CC = crate::interrupt::typelevel::TIM4;
        pub type COM = crate::interrupt::typelevel::TIM4;
        pub type TRG = crate::interrupt::typelevel::TIM4;
        pub type UP = crate::interrupt::typelevel::TIM4;
    }
    pub mod TIM6 {
        pub type BRK = crate::interrupt::typelevel::TIM6_DAC;
        pub type CC = crate::interrupt::typelevel::TIM6_DAC;
        pub type COM = crate::interrupt::typelevel::TIM6_DAC;
        pub type TRG = crate::interrupt::typelevel::TIM6_DAC;
        pub type UP = crate::interrupt::typelevel::TIM6_DAC;
    }
    pub mod TIM7 {
        pub type BRK = crate::interrupt::typelevel::TIM7;
        pub type CC = crate::interrupt::typelevel::TIM7;
        pub type COM = crate::interrupt::typelevel::TIM7;
        pub type TRG = crate::interrupt::typelevel::TIM7;
        pub type UP = crate::interrupt::typelevel::TIM7;
    }
    pub mod TIM8 {
        pub type BRK = crate::interrupt::typelevel::TIM8_BRK;
        pub type CC = crate::interrupt::typelevel::TIM8_CC;
        pub type COM = crate::interrupt::typelevel::TIM8_TRG_COM;
        pub type TRG = crate::interrupt::typelevel::TIM8_TRG_COM;
        pub type UP = crate::interrupt::typelevel::TIM8_UP;
    }
    pub mod UART4 {
        pub type GLOBAL = crate::interrupt::typelevel::UART4;
    }
    pub mod UCPD1 {
        pub type GLOBAL = crate::interrupt::typelevel::UCPD1;
    }
    pub mod UID {}
    pub mod USART1 {
        pub type GLOBAL = crate::interrupt::typelevel::USART1;
    }
    pub mod USART2 {
        pub type GLOBAL = crate::interrupt::typelevel::USART2;
    }
    pub mod USART3 {
        pub type GLOBAL = crate::interrupt::typelevel::USART3;
    }
    pub mod USB {
        pub type HP = crate::interrupt::typelevel::USB_HP;
        pub type LP = crate::interrupt::typelevel::USB_LP;
        pub type WKUP = crate::interrupt::typelevel::USBWAKEUP;
    }
    pub mod USBRAM {}
    pub mod VREFBUF {}
    pub mod WWDG {
        pub type GLOBAL = crate::interrupt::typelevel::WWDG;
        pub type RST = crate::interrupt::typelevel::WWDG;
    }
}
dma_channel_impl!(DMA1_CH1, 0u8);
dma_channel_impl!(DMA1_CH2, 1u8);
dma_channel_impl!(DMA1_CH3, 2u8);
dma_channel_impl!(DMA1_CH4, 3u8);
dma_channel_impl!(DMA1_CH5, 4u8);
dma_channel_impl!(DMA1_CH6, 5u8);
dma_channel_impl!(DMA2_CH1, 6u8);
dma_channel_impl!(DMA2_CH2, 7u8);
dma_channel_impl!(DMA2_CH3, 8u8);
dma_channel_impl!(DMA2_CH4, 9u8);
dma_channel_impl!(DMA2_CH5, 10u8);
dma_channel_impl!(DMA2_CH6, 11u8);
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA1_CHANNEL1() {
    <crate::peripherals::DMA1_CH1 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA1_CHANNEL2() {
    <crate::peripherals::DMA1_CH2 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA1_CHANNEL3() {
    <crate::peripherals::DMA1_CH3 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA1_CHANNEL4() {
    <crate::peripherals::DMA1_CH4 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA1_CHANNEL5() {
    <crate::peripherals::DMA1_CH5 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA1_CHANNEL6() {
    <crate::peripherals::DMA1_CH6 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA2_CHANNEL1() {
    <crate::peripherals::DMA2_CH1 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA2_CHANNEL2() {
    <crate::peripherals::DMA2_CH2 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA2_CHANNEL3() {
    <crate::peripherals::DMA2_CH3 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA2_CHANNEL4() {
    <crate::peripherals::DMA2_CH4 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA2_CHANNEL5() {
    <crate::peripherals::DMA2_CH5 as crate::dma::ChannelInterrupt>::on_irq();
}
#[cfg(feature = "rt")]
#[crate::interrupt]
unsafe fn DMA2_CHANNEL6() {
    <crate::peripherals::DMA2_CH6 as crate::dma::ChannelInterrupt>::on_irq();
}
pub(crate) const DMA_CHANNELS: &[crate::dma::ChannelInfo] = &[
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA1),
        num: 0usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 0usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA1),
        num: 1usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 1usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA1),
        num: 2usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 2usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA1),
        num: 3usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 3usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA1),
        num: 4usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 4usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA1),
        num: 5usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 5usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA2),
        num: 0usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 8usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA2),
        num: 1usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 9usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA2),
        num: 2usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 10usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA2),
        num: 3usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 11usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA2),
        num: 4usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 12usize,
        },
    },
    crate::dma::ChannelInfo {
        dma: crate::dma::DmaInfo::Bdma(crate::pac::DMA2),
        num: 5usize,
        dmamux: crate::dma::DmamuxInfo {
            mux: crate::pac::DMAMUX1,
            num: 13usize,
        },
    },
];
pub fn gpio_block(n: usize) -> crate::pac::gpio::Gpio {
    unsafe { crate::pac::gpio::Gpio::from_ptr((1207959552usize + 1024usize * n) as _) }
}
pub const FLASH_BASE: usize = 134217728usize;
pub const FLASH_SIZE: usize = 131072usize;
pub const WRITE_SIZE: usize = 8usize;
