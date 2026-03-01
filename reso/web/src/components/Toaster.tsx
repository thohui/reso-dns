import {
	Toaster as ChakraToaster,
	createToaster,
	Portal,
	Spinner,
	Stack,
	Toast,
} from '@chakra-ui/react';
import { getApiError } from '../lib/api/error';

export const toaster = createToaster({
	placement: 'bottom-end',
	pauseOnPageIdle: true,
});

export async function toastError(e: unknown) {
	const toasterDuration = 3000;

	const error = await getApiError(e);

	if (error) {
		toaster.error({
			title: 'Error',
			description: error.message,
			duration: toasterDuration,
		});
	} else if (e instanceof Error) {
		toaster.error({
			title: 'Error',
			description: e.message,
			duration: toasterDuration,
		});
	} else {
		toaster.error({
			title: 'Error',
			description: 'Something went wrong',
			duration: toasterDuration,
		});
	}
}

export const Toaster = () => {
	return (
		<Portal>
			<ChakraToaster toaster={toaster} insetInline={{ mdDown: '4' }}>
				{(toast) => (
					<Toast.Root width={{ md: 'sm' }}>
						{toast.type === 'loading' ? (
							<Spinner size='sm' color='blue.solid' />
						) : (
							<Toast.Indicator />
						)}
						<Stack gap='1' flex='1' maxWidth='100%'>
							{toast.title && <Toast.Title>{toast.title}</Toast.Title>}
							{toast.description && (
								<Toast.Description>{toast.description}</Toast.Description>
							)}
						</Stack>
						{toast.action && (
							<Toast.ActionTrigger>{toast.action.label}</Toast.ActionTrigger>
						)}
						{toast.closable && <Toast.CloseTrigger />}
					</Toast.Root>
				)}
			</ChakraToaster>
		</Portal>
	);
};
