import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(''),
  routes: [
    {
      path: '',
      name: 'home',
      component: () => import('./features/home/Home.vue'),
    },
    {
      path: '/contexts',
      name: 'contexts',
      component: () => import('./features/contexts/Contexts.vue'),
    },
    {
      path: '/contexts/create',
      name: 'createContext',
      component: () => import('./features/contexts/create/CreateContextPage.vue'),
    },
    {
      path: '/contexts/:contextId/edit',
      name: 'context-edit',
      component: () => import('./features/contexts/edit/EditContext.vue'),
    },
    {
      path: '/machines',
      name: 'machines',
      component: () => import('./features/machines/Machines.vue'),
    },
    {
      path: '/machines/edit/:machineId',
      name: 'machineEdit',
      component: () => import('./features/machines/edit/MachineEdit.vue'),
    },
    {
      path: '/images',
      name: 'images',
      component: () => import('./features/images/Images.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('./features/settings/Settings.vue'),
    },
    {
      path: '/invite/:inviteUrl',
      name: 'invite',
      component: () => import('./features/invite/Invite.vue'),
    },
  ],
})

export default router
