import {createRouter, createWebHistory} from 'vue-router'

const routes = [
    {
        path: '/',
        name: 'Home',
        component: () => import('./components/Greet.vue')
    }, {
        path: '/sample/:pid',
        name: 'Sample',
        component: () => import('./components/Sample.vue'),
    }
]

export default createRouter({
    history: createWebHistory(),
    routes
})