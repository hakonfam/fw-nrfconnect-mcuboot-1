if(CONFIG_BOOTLOADER_MCUBOOT)
  # Build a second bootloader image

  set(MCUBOOT_BASE ${CMAKE_CURRENT_LIST_DIR}/..)

  zephyr_add_executable(mcuboot require_build)

  if (${require_build})
    add_subdirectory(${MCUBOOT_BASE}/boot/zephyr ${CMAKE_CURRENT_BINARY_DIR}/mcuboot)

    # TODO: Assert that the bootloader and image use the same key.

    get_property(mcuboot_sign_merged GLOBAL PROPERTY MCUBOOT_SIGN_MERGED)
    if (mcuboot_sign_merged)
      message("HALLELUJA")
      set(mcuboot_sign_depends MERGED_TO_SIGN_TARGET)
      set(to_sign ${CMAKE_BINARY_DIR}/${mcuboot_sign_merged})
    else()
      set(to_sign ${KERNEL_HEX_NAME})
    endif()

    set(SIGNED_IMAGE signed.hex)

    add_custom_command(
      OUTPUT ${SIGNED_IMAGE}
      COMMAND
      ${PYTHON_EXECUTABLE}
      ${MCUBOOT_BASE}/scripts/imgtool.py
      sign
      --key ${MCUBOOT_BASE}/${CONFIG_BOOT_SIGNATURE_KEY_FILE}
      --header-size ${CONFIG_TEXT_SECTION_OFFSET}
      --align       ${DT_FLASH_WRITE_BLOCK_SIZE}
      --version 0.1       # TODO: Configurable?
      --slot-size 0x69000 # TODO: Configurable?
      ${to_sign}
      ${SIGNED_IMAGE}
      WORKING_DIRECTORY
      ${CMAKE_BINARY_DIR}
      DEPENDS
      ${mcuboot_sign_depends}
      )

    print(ZEPHYR_BINARY_DIR)
    print(CMAKE_BINARY_DIR)
    print(PROJECT_BINARY_DIR)
    add_custom_target(signed_image_target DEPENDS ${SIGNED_IMAGE})
    print(SIGNED_IMAGE)

    set_property(GLOBAL APPEND PROPERTY
      HEX_FILES_TO_MERGE
      ${CMAKE_BINARY_DIR}/${SIGNED_IMAGE}
      )
    set_property(GLOBAL APPEND PROPERTY
      HEX_FILES_TO_MERGE_TARGET
      signed_image_target
      )
  endif() # ${require_build}
endif()
