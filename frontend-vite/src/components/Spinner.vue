/*
    Shows a wait spinner to the user, letting them know an operation is occurring
    Properties:
        display - set this when the spinner should be displayed
*/
<script setup lang="ts">
    import Modal from "./Modal.vue";

    const props = defineProps({
        display: Boolean,
    });
</script>

<template>
        <Modal class="spinner_back"
            :display="display"
        >
            <div class="spinner"></div>
        </Modal>
</template>

<style scoped>
/* loader from - https://projects.lukehaas.me/css-loaders/ */

.spinner, .spinner:after {
  border-radius: 50%;
  width: 10em;
  height: 10em;
}
.spinner {
  margin: 200px auto;
  font-size: 10px;
  position: relative;
  text-indent: -9999em;
  border-top: 1.1em solid rgba(255, 255, 255, 0.2);
  border-right: 1.1em solid rgba(255, 255, 255, 0.2);
  border-bottom: 1.1em solid rgba(255, 255, 255, 0.2);
  border-left: 1.1em solid #ffffff;
  transform: translateZ(0);
  animation: spinner-load8 1.1s infinite linear;
}
@keyframes spinner-load8 {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}
</style>
