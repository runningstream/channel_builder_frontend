/*
    Provides a way for the user to add a new channel
    Properties:
        display - set this when the dialog should be displayed
    Emits:
        cancelModal - when this event is emitted, the parent should unset/falisfy the display property and ignore any data entered
        saveModal - when this event is emitted, the parent should unset/falisfy the display property and save any data entered
*/
<script setup lang="ts">
    import { ref, watch } from "vue";
    import Modal from "./Modal.vue";

    import type { Ref } from "vue";

    const props = defineProps({
        display: Boolean,
    });

    const name : Ref<string> = ref("");

    const emit = defineEmits<{
        (e: "cancelModal") : void
        (e: "saveModal", name: string) : void
    }>();

    // Reset the text entry box
    watch( () => props.display, (disp_val) => {
        if(disp_val) {
            name.value = "";
        }
    })

    function save_entry() {
        emit('saveModal', name.value);
    }
</script>

<template>
        <Modal class="demo_modal"
            :display="display"
            @pressedEsc="$emit('cancelModal')"
        >
            <form class="newchannel_box" @submit.prevent="save_entry()">
                <div class="close_button" @click="$emit('cancelModal')">x</div>
                <div class="inputs_area">
                    <label>Name: <input type="text" v-model="name"></label>
                </div>
                <div class="button_area">
                    <input type="submit" class="bigger_text" value="New Channel">
                    <input type="button" class="bigger_text" @click="$emit('cancelModal')" value="Cancel">
                </div>
            </form>
        </Modal>
</template>

<style scoped>
.newchannel_box {
    position: relative;
    width: fit-content;
    max-width: 90%;
    margin: 4em auto;
    padding: 2em;
    padding-bottom: 1em;
    background-color: var(--color-ltolive);
    border-radius: 20px;
    text-align: center;
}
.close_button {
    top: .4em;
    right: .4em;
    position: absolute;
    font-family: sans-serif;
    cursor: pointer;
}
.button_area {
    padding-top: 1em;
}
.inputs_area label {
    display: block;
}
.inputs_area input[type=text] {
    width: 40em;
    max-width: 90%;
}
</style>
