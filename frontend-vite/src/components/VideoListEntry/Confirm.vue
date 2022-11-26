/*
    Provides a text confirmation to the user, displayed in a modal, with an ok button.
    Properties:
        text - the text to display in the alert
        display - set this when the alert should be displayed
    Emits:
        okModal - when this event is emitted, the parent should unset/falsify the display property, but also the user has clicked "ok"
        cancelModal - when this event is emitted, the parent should unset/falisfy the display property, and the user has clicked "cancel"
*/
<script setup lang="ts">
    import Modal from "../Modal.vue";

    const props = defineProps({
        text: String,
        display: Boolean,
    });

    const emit = defineEmits(["cancelModal", "okModal"]);
</script>

<template>
        <Modal
            :display="display"
            @pressedEnt="$emit('cancelModal')"
            @pressedEsc="$emit('cancelModal')"
            @clickedOff="$emit('cancelModal')"
        >
            <form class="confirm_box" @click.stop="" @submit.prevent="">
                <div class="close_button" @click="$emit('cancelModal')">x</div>
                <div>{{ text }}</div>
                <div class="button_area">
                    <input type="button" class="bigger_text" @click="$emit('okModal')" value="Ok" />
                    <input type="submit" class="bigger_text" @click="$emit('cancelModal')" value="Cancel" />
                </div>
            </form>
        </Modal>
</template>

<style scoped>
.confirm_box {
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
