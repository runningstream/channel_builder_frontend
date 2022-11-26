/*
    Provides a way for the user to add or edit a sublist
    Properties:
        inputEntry - set this to a VideoType object when the user should modify an existing entry
        display - set this when the dialog should be displayed
    Emits:
        cancelModal - when this event is emitted, the parent should unset/falisfy the display property and ignore any data entered
        saveModal - when this event is emitted, the parent should unset/falisfy the display property and save any data entered
*/
<script setup lang="ts">
    import { ref, watch } from "vue";
    import Modal from "../Modal.vue";
    import { newVideoTypeSublist } from "../../api_js/serverAPI";

    import type { Ref } from "vue";
    import type { VideoType } from "../../api_js/serverAPI";

    const props = defineProps({
        inputEntry: Object as () => VideoType | null,
        display: Boolean,
    });

    const emit = defineEmits<{
        (e: "cancelModal") : void
        (e: "saveModal", entry: VideoType) : void
    }>();

    // entry acts as a temporary version of the entry we're modifying
    const entry: Ref<VideoType> = ref({});

    // When the modal is displayed we need to update the temporary entry
    // so this watcher looks for when props.display is set true and runs
    watch( () => {return props.display;}, (newDisp, _oldDisp) => {
        if( newDisp ) {
            setup_entry();
        }
    });

    // Set a default temp entry or copy in the provided one
    // Additionally, URLs need to be decoded before shown to user
    function setup_entry() {
        entry.value = newVideoTypeSublist();
        if( props.inputEntry != undefined ) {
            entry.value.name = props.inputEntry.name;
            entry.value.image = decodeURI(props.inputEntry.image ? props.inputEntry.image : "");
            entry.value.entries = props.inputEntry.entries;
        }
    }

    function save_entry() {
        entry.value.image = encodeURI(entry.value.image ? entry.value.image : "");
        emit('saveModal', entry.value);
    }
</script>

<template>
        <Modal class="demo_modal"
            :display="display"
            @pressedEsc="$emit('cancelModal')"
        >
            <form class="sublistedit_box" @submit.prevent="save_entry()">
                <div class="close_button" @click="$emit('cancelModal')">x</div>
                <div class="inputs_area">
                    <label>Name: <input type="text" v-model="entry.name"></label>
                    <label>Image URL: <input type="text" v-model="entry.image"></label>
                </div>
                <div class="button_area">
                    <input type="submit" class="bigger_text" :value="props.inputEntry == undefined ? 'Add Sublist' : 'Modify Sublist'">
                    <input type="button" class="bigger_text" @click="$emit('cancelModal')" value="Cancel">
                </div>
            </form>
        </Modal>
</template>

<style scoped>
.sublistedit_box {
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
