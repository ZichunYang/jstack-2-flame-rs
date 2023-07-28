<script setup>
import {ref} from "vue";
import {invoke} from "@tauri-apps/api/tauri";

const processes = ref([]);

async function greet() {
    processes.value = await invoke("greet", {});
}

await greet();

</script>

<template>
    <a-typography>
        <a-typography-title :level="3">JAVA</a-typography-title>

        <a-list item-layout="horizontal" :data-source="processes">
            <template #renderItem="{ item }">
                <a-list-item>
                    <template #actions>
                        <a href="#">
                            <router-link :to="'/sample/' + item.pid">
                                Sample
                            </router-link>
                        </a>
                    </template>
                    <a-list-item-meta :description="'PID: ' + item.pid">
                        <template #title>
                            <a href="#">
                                {{ item.name }}
                            </a>
                        </template>
                        <template #avatar>
                            <a-avatar
                                src="/icons8-java-48.png"/>
                        </template>
                    </a-list-item-meta>
                </a-list-item>
            </template>
        </a-list>

        <div style="margin-top: 20px; width: 100%; display: flex; align-items: center; justify-content: center;">
            <a-button type="primary" @click="greet">Refresh</a-button>
        </div>
    </a-typography>
</template>