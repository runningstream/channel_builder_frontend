<script setup lang="ts">
    import { ref } from "vue";
    import Modal from "./Modal.vue";

    const props = defineProps({
        img: String,
        text: String,
        usecase_gif: String,
    });

    const demo_displayed = ref(false);

    function select_usecase() {
        if( props.usecase_gif != undefined ) {
            demo_displayed.value = true;
        }
    }
    
</script>

<template>
    <div class="usecase" @click="select_usecase()">
        <img class="uc_icon_img" :src="img" />
        <p>{{ text }}</p>
        <Modal class="demo_modal"
            v-if="usecase_gif != undefined"
            :display="demo_displayed"
            @clickedOff="demo_displayed=false"
            @pressedEsc="demo_displayed=false"
        >
            <img :src=" usecase_gif">
        </Modal>
    </div>
</template>

<style scoped>
.uc_icon_img {
    width: 100%;
    filter: invert(86%) sepia(28%) saturate(126%) hue-rotate(23deg) brightness(109%) contrast(82%);
}
.usecase p {
    margin: 0;
}
.demo_modal img {
    margin: 4em auto;
    border: 5px solid black;
    max-width: 90%;
}
</style>
