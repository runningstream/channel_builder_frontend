/*
    Provides a text alert to the user, displayed in a modal, with an ok button.
    Properties:
        text - the text to display in the alert
        display - set this when the alert should be displayed
    Emits:
        closeModal - when this event is emitted, the parent should unset/falisfy the display property
*/
<script setup lang="ts">
    import Modal from "./Modal.vue";

    const props = defineProps({
        text: String,
        display: Boolean,
    });

    const emit = defineEmits(["closeModal"]);
</script>

<template>
        <Modal class="demo_modal"
            :display="display"
            @pressedEnt="$emit('closeModal')"
            @pressedEsc="$emit('closeModal')"
            @clickedOff="$emit('closeModal')"
        >
            <form class="alert_box" @click.stop="" @submit.prevent="">
                <div class="close_button" @click="$emit('closeModal')">x</div>
                <div>{{ text }}</div>
                <div class="button_area">
                    <input type="submit" class="bigger_text" @click="$emit('closeModal')" value="Ok" />
                </div>
            </form>
        </Modal>
</template>

<style scoped>
.alert_box {
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
</style>
