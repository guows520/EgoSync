package com.egosync.companion

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import com.egosync.companion.ui.AppNavHost
import com.egosync.companion.ui.theme.EgoSyncTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val container = AppModelContainer(applicationContext)
        setContent {
            EgoSyncRoot(container)
        }
    }
}

@Composable
private fun EgoSyncRoot(container: AppModelContainer) {
    val themeMode by container.themeMode.collectAsState()
    EgoSyncTheme(themeMode = themeMode) {
        val snackbarHostState = remember { SnackbarHostState() }
        val eventMessage by container.eventMessage.collectAsState()

        LaunchedEffect(eventMessage) {
            eventMessage?.let {
                snackbarHostState.showSnackbar(it)
                container.consumeEvent()
            }
        }

        Scaffold(
            snackbarHost = { SnackbarHost(snackbarHostState) },
            containerColor = MaterialTheme.colorScheme.background,
        ) { padding ->
            Box(
                Modifier
                    .fillMaxSize()
                    .padding(padding)
            ) {
                AppNavHost(container = container)
            }
        }
    }
}
