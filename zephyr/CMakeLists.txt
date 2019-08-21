if(CONFIG_BOOTLOADER_MCUBOOT)
  set(test_update_hex ${PROJECT_BINARY_DIR}/test_update.hex)
  function(sign to_sign_hex output_prefix offset sign_depends signed_hex_out)
    set(op ${output_prefix})
    set(signed_hex ${op}_signed.hex)
    set(${signed_hex_out} ${signed_hex} PARENT_SCOPE)
    set(to_sign_bin ${op}_to_sign.bin)
    set(update_hex ${op}_update.hex)
    set(update_bin ${op}_update.bin)
    set(moved_update_hex ${op}_moved_update.hex)

    add_custom_command(
      OUTPUT
      ${signed_hex}
      ${update_hex}
      ${update_bin}
      COMMAND
      ${sign_cmd}
      ${to_sign_hex}
      ${signed_hex}
      COMMAND
      ${CMAKE_OBJCOPY}
      --input-target=ihex
      --output-target=binary
      ${to_sign_hex}
      ${to_sign_bin}
      COMMAND
      ${sign_cmd}
      ${to_sign_bin}
      ${update_bin}
      COMMAND
      ${sign_cmd}
      --pad # Note: Adds IMAGE_MAGIC to end of slot.
    ${to_sign_hex}
    ${test_update_hex}
    COMMAND
    ${sign_cmd}
      ${to_sign_hex}
      ${update_hex}
      COMMAND
      ${CMAKE_OBJCOPY}
      --input-target=ihex
      --output-target=ihex
      --change-address ${offset}
      ${update_hex}
      ${moved_update_hex}
      DEPENDS
      ${sign_depends}
      )
  endfunction()

  # Build a second bootloader image
  set(MCUBOOT_BASE ${CMAKE_CURRENT_LIST_DIR}/..)

  zephyr_add_executable(mcuboot require_build)

  if (${require_build})
    add_subdirectory(${MCUBOOT_BASE}/boot/zephyr ${CMAKE_BINARY_DIR}/mcuboot)
  endif() # ${require_build}

  set(app_to_sign_hex ${KERNEL_HEX_NAME})

  # TODO: Assert that the bootloader and image use the same key.

  get_property(app_binary_dir GLOBAL PROPERTY PROJECT_BINARY_DIR)
  set(merged_hex_file
    ${app_binary_dir}/mcuboot_primary_app.hex)
  set(merged_hex_file_depends
    mcuboot_primary_app_hex$<SEMICOLON>${PROJECT_BINARY_DIR}/mcuboot_primary_app.hex)
  set(sign_merged
    $<TARGET_EXISTS:partition_manager>)
  set(app_to_sign_hex
    $<IF:${sign_merged},${merged_hex_file},${PROJECT_BINARY_DIR}/${KERNEL_HEX_NAME}>)
  set(app_sign_depends
    $<IF:${sign_merged},${merged_hex_file_depends},zephyr_final>)

  set(sign_cmd
    ${PYTHON_EXECUTABLE}
    ${MCUBOOT_BASE}/scripts/imgtool.py
    sign
    --key ${MCUBOOT_BASE}/${CONFIG_BOOT_SIGNATURE_KEY_FILE}
    --header-size $<TARGET_PROPERTY:partition_manager,PM_MCUBOOT_PAD_SIZE>
    --align       ${DT_FLASH_WRITE_BLOCK_SIZE}
    --version     ${CONFIG_MCUBOOT_IMAGE_VERSION}
    --slot-size   $<TARGET_PROPERTY:partition_manager,PM_MCUBOOT_PRIMARY_SIZE>
    --pad-header
    )

  set(app_offset $<TARGET_PROPERTY:partition_manager,PM_MCUBOOT_PRIMARY_SIZE>)

  sign(${app_to_sign_hex}     # Hex to sign
    ${PROJECT_BINARY_DIR}/app # Prefix for generated files
    ${app_offset}             # Offset
    ${app_sign_depends}       # Dependencies
    app_signed_hex            # Generated hex output variable
    )

  add_custom_target(mcuboot_sign_target DEPENDS ${app_signed_hex})

  set_property(GLOBAL PROPERTY
    mcuboot_primary_app_PM_HEX_FILE
    ${app_signed_hex}
    )
  set_property(GLOBAL PROPERTY
    mcuboot_primary_app_PM_TARGET
    mcuboot_sign_target
    )
endif()
