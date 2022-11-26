/*
    A modal that can include any contents
    Properties:
        display - set this when the modal should be displayed
    Emits:
        clickedOff - when this event is emitted, the user clicked off of the modal
        pressedEsc - when this event is emitted, the user pressed escape
        pressedEnt - when this event is emitted, the user pressed enter
    Slot: the contents
*/
<script setup lang="ts">
    import { ref, watch } from "vue";
    import type { Ref } from "vue";

    const modal_elem: Ref<HTMLDivElement | null> = ref(null);

    const props = defineProps({
        display: Boolean,
    });
    const emit = defineEmits(["clickedOff", "pressedEsc", "pressedEnt"]);

    // This watcher makes sure that focus cannot leave the modal while the modal
    // is displayed
    watch( () => props.display, () => {
        function focuser( event: FocusEvent ) : void {
            if( modal_elem.value == null ) {
                return;
            }
            if( modal_elem.value !== event.target && 
                !modal_elem.value.contains( event.target as Node | null ))
            {
                modal_elem.value.focus();
            }
        }

        if( props.display ) {
            document.addEventListener("focusin", focuser);
            if( modal_elem.value != null ) { modal_elem.value.focus() };
        } else {
            document.removeEventListener("focusin", focuser);
        }
    });
</script>

<template>
    <div class="modal" :class="{ modal_display: display }"
        tabindex="-1"
        @click.stop="$emit('clickedOff')"
        @keydown.esc.stop="$emit('pressedEsc')"
        @keydown.ent="$emit('pressedEnt')"
        ref="modal_elem"
    >
        <slot />
    </div>
</template>

<style scoped>
.modal {
    display: none;
    position: fixed;
    z-index: 100;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(var(--color-dkscarlet-rgb), .2);
    backdrop-filter: blur(2px);
}
.modal_display {
    display: block;
}
</style>
