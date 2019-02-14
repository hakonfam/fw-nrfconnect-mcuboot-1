#define _EXTERNAL_LIBOBERON .extabis_oberon
#define _EXTERNAL_API_NAME .extabis

SECTION_PROLOGUE(_EXTERNAL_LIBOBERON,,)
{
	KEEP(*(.data.inv_a))
	KEEP(*(".data.inv_a.*"))
} GROUP_LINK_IN(SHARED_SRAM)

SECTION_PROLOGUE(_EXTERNAL_API_NAME,,)
{
	. = ALIGN(4);
	_abis_start = .;
	KEEP(*(.extabis))
	KEEP(*(".extabis.*"))
	_abis_end = .;
} GROUP_LINK_IN(ROMABLE_REGION)
